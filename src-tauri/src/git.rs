use git2::Repository;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize)]
pub struct Worktree {
    pub name: String,
    pub path: String,
    pub branch: Option<String>,
    pub is_default: bool,
}

#[tauri::command]
pub async fn get_git_remote_url(path: String) -> Result<Option<String>, String> {
    let repo_path = Path::new(&path);

    if !is_repo_root(repo_path) {
        return Ok(None);
    }

    let repository = match Repository::open(repo_path) {
        Ok(repo) => repo,
        Err(_) => return Ok(None),
    };

    let remote = match repository.find_remote("origin") {
        Ok(remote) => remote,
        Err(_) => return Ok(None),
    };

    let url = remote.url().map(|u| normalize_remote_url(u));
    Ok(url)
}

fn is_repo_root(path: &Path) -> bool {
    path.join(".git").exists()
}

#[tauri::command]
pub async fn check_is_git_repository(path: String) -> Result<bool, String> {
    Ok(is_repo_root(Path::new(&path)))
}

#[tauri::command]
pub async fn list_worktrees(path: String) -> Result<Vec<Worktree>, String> {
    let repo_path = Path::new(&path);

    let repository = Repository::open(repo_path).map_err(|e| e.to_string())?;

    let worktree_names = repository.worktrees().map_err(|e| e.to_string())?;

    let mut worktrees: Vec<Worktree> = Vec::new();

    let main_branch = get_head_branch(&repository);
    let main_path = repository
        .workdir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    worktrees.push(Worktree {
        name: "default".to_string(),
        path: normalize_path(&main_path),
        branch: main_branch,
        is_default: true,
    });

    for i in 0..worktree_names.len() {
        if let Some(name) = worktree_names.get(i) {
            if let Ok(wt) = repository.find_worktree(name) {
                let wt_path = wt.path().to_string_lossy().to_string();
                let branch = get_worktree_branch(&repository, &wt_path);
                worktrees.push(Worktree {
                    name: name.to_string(),
                    path: normalize_path(&wt_path),
                    branch,
                    is_default: false,
                });
            }
        }
    }

    Ok(worktrees)
}

fn get_head_branch(repo: &Repository) -> Option<String> {
    repo.head()
        .ok()
        .and_then(|head| head.shorthand().map(|s| s.to_string()))
}

fn get_worktree_branch(repo: &Repository, worktree_path: &str) -> Option<String> {
    let wt_repo = Repository::open(worktree_path).ok()?;
    get_head_branch(&wt_repo)
}

fn normalize_path(path: &str) -> String {
    if path.ends_with('/') {
        path[..path.len() - 1].to_string()
    } else {
        path.to_string()
    }
}

fn normalize_remote_url(url: &str) -> String {
    if url.starts_with("git@") {
        let without_prefix = url.strip_prefix("git@").unwrap_or(url);
        let normalized = without_prefix.replace(':', "/");
        let without_suffix = normalized.strip_suffix(".git").unwrap_or(&normalized);
        format!("https://{}", without_suffix)
    } else if url.ends_with(".git") {
        url.strip_suffix(".git").unwrap_or(url).to_string()
    } else {
        url.to_string()
    }
}

#[derive(Serialize)]
pub struct WorktreeStatus {
    pub files_added: usize,
    pub files_modified: usize,
    pub files_deleted: usize,
    pub files_renamed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub pr_url: Option<String>,
}

#[tauri::command]
pub async fn get_worktree_status(path: String) -> Result<WorktreeStatus, String> {
    let repo = Repository::open(&path).map_err(|e| e.to_string())?;

    let changed_files = count_changed_files(&repo)?;
    let (insertions, deletions) = count_changed_lines(&repo);
    let pr_url = build_pr_url(&repo);

    Ok(WorktreeStatus {
        files_added: changed_files.added,
        files_modified: changed_files.modified,
        files_deleted: changed_files.deleted,
        files_renamed: changed_files.renamed,
        insertions,
        deletions,
        pr_url,
    })
}

struct FileChangeCounts {
    added: usize,
    modified: usize,
    deleted: usize,
    renamed: usize,
}

fn count_changed_files(repo: &Repository) -> Result<FileChangeCounts, String> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true);
    opts.exclude_submodules(true);

    let statuses = repo.statuses(Some(&mut opts)).map_err(|e| e.to_string())?;

    let mut counts = FileChangeCounts {
        added: 0,
        modified: 0,
        deleted: 0,
        renamed: 0,
    };

    for entry in statuses.iter() {
        let status = entry.status();
        if status.intersects(git2::Status::WT_NEW | git2::Status::INDEX_NEW) {
            counts.added += 1;
        } else if status.intersects(git2::Status::WT_DELETED | git2::Status::INDEX_DELETED) {
            counts.deleted += 1;
        } else if status.intersects(git2::Status::WT_RENAMED | git2::Status::INDEX_RENAMED) {
            counts.renamed += 1;
        } else if status.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::INDEX_MODIFIED
                | git2::Status::WT_TYPECHANGE
                | git2::Status::INDEX_TYPECHANGE,
        ) {
            counts.modified += 1;
        }
    }

    Ok(counts)
}

fn count_changed_lines(repo: &Repository) -> (usize, usize) {
    let diff = match repo.diff_index_to_workdir(None, None) {
        Ok(d) => d,
        Err(_) => return (0, 0),
    };

    let stats = match diff.stats() {
        Ok(s) => s,
        Err(_) => return (0, 0),
    };

    (stats.insertions(), stats.deletions())
}

fn build_pr_url(repo: &Repository) -> Option<String> {
    let remote = repo.find_remote("origin").ok()?;
    let remote_url = remote.url()?;
    let base_url = normalize_remote_url(remote_url);

    if !base_url.contains("github.com") {
        return None;
    }

    let branch = get_head_branch(repo)?;
    Some(format!("{}/compare/{}?expand=1", base_url, branch))
}

#[tauri::command]
pub async fn list_branches(path: String) -> Result<Vec<String>, String> {
    let repo = Repository::open(&path).map_err(|e| e.to_string())?;

    let branches = repo
        .branches(Some(git2::BranchType::Local))
        .map_err(|e| e.to_string())?;

    let mut branch_names: Vec<String> = Vec::new();
    for branch in branches {
        let (branch, _) = branch.map_err(|e| e.to_string())?;
        if let Some(name) = branch.name().map_err(|e| e.to_string())? {
            branch_names.push(name.to_string());
        }
    }

    branch_names.sort();
    Ok(branch_names)
}

#[tauri::command]
pub async fn create_worktree(
    path: String,
    branch_name: String,
    base_branch: String,
) -> Result<String, String> {
    let repo = Repository::open(&path).map_err(|e| e.to_string())?;

    let base_commit = {
        let base_ref = repo
            .find_branch(&base_branch, git2::BranchType::Local)
            .map_err(|e| format!("Branch '{}' not found: {}", base_branch, e))?;
        base_ref.get().peel_to_commit().map_err(|e| e.to_string())?
    };

    repo.branch(&branch_name, &base_commit, false)
        .map_err(|e| format!("Failed to create branch '{}': {}", branch_name, e))?;

    let worktree_dir = repo
        .workdir()
        .ok_or("Could not determine workdir")?
        .parent()
        .ok_or("Could not determine parent directory")?
        .join(&branch_name);

    let worktree_path = worktree_dir.to_string_lossy().to_string();

    std::process::Command::new("git")
        .args(["worktree", "add", &worktree_path, &branch_name])
        .current_dir(&path)
        .output()
        .map_err(|e| format!("Failed to create worktree: {}", e))?;

    Ok(normalize_path(&worktree_path))
}

#[tauri::command]
pub async fn delete_worktree(repo_path: String, worktree_path: String) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .args(["worktree", "remove", "--force", &worktree_path])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("Failed to remove worktree: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git worktree remove failed: {}", stderr));
    }

    let wt_path = std::path::Path::new(&worktree_path);
    if wt_path.exists() {
        std::fs::remove_dir_all(wt_path)
            .map_err(|e| format!("Failed to delete worktree directory: {}", e))?;
    }

    Ok(())
}

#[derive(Deserialize)]
pub struct ContextRepoInput {
    pub rel_path: String,
    pub name: String,
    pub branch: String,
    pub base_branch: String,
    #[serde(default)]
    pub linked: bool,
}

#[derive(Serialize)]
pub struct ContextRepoResult {
    pub rel_path: String,
    pub worktree_path: String,
    pub branch: String,
}

#[tauri::command]
pub async fn create_context(
    project_path: String,
    context_name: String,
    repos: Vec<ContextRepoInput>,
) -> Result<Vec<ContextRepoResult>, String> {
    let project_dir = Path::new(&project_path);
    let mut results: Vec<ContextRepoResult> = Vec::new();

    for repo_input in &repos {
        let repo_path = project_dir.join(&repo_input.rel_path);
        let worktree_dir = project_dir
            .join(".worktrees")
            .join(&context_name)
            .join(&repo_input.name);

        if repo_input.linked {
            if let Some(parent) = worktree_dir.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create parent directory: {}", e)
                })?;
            }

            #[cfg(unix)]
            std::os::unix::fs::symlink(&repo_path, &worktree_dir).map_err(|e| {
                format!(
                    "Failed to create symlink for '{}': {}",
                    repo_input.name, e
                )
            })?;

            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(&repo_path, &worktree_dir).map_err(|e| {
                format!(
                    "Failed to create symlink for '{}': {}",
                    repo_input.name, e
                )
            })?;

            results.push(ContextRepoResult {
                rel_path: repo_input.rel_path.clone(),
                worktree_path: normalize_path(&worktree_dir.to_string_lossy()),
                branch: repo_input.branch.clone(),
            });
        } else {
            let repo_path_str = repo_path.to_string_lossy().to_string();

            let repo = Repository::open(&repo_path)
                .map_err(|e| format!("Failed to open repo at '{}': {}", repo_input.rel_path, e))?;

            let branch_exists = repo
                .find_branch(&repo_input.branch, git2::BranchType::Local)
                .is_ok();

            if !branch_exists {
                let base_commit = {
                    let base_ref = repo
                        .find_branch(&repo_input.base_branch, git2::BranchType::Local)
                        .map_err(|e| {
                            format!(
                                "Base branch '{}' not found in '{}': {}",
                                repo_input.base_branch, repo_input.rel_path, e
                            )
                        })?;
                    base_ref.get().peel_to_commit().map_err(|e| e.to_string())?
                };

                repo.branch(&repo_input.branch, &base_commit, false)
                    .map_err(|e| {
                        format!(
                            "Failed to create branch '{}' in '{}': {}",
                            repo_input.branch, repo_input.rel_path, e
                        )
                    })?;
            }

            let worktree_path = worktree_dir.to_string_lossy().to_string();

            let output = std::process::Command::new("git")
                .args(["worktree", "add", &worktree_path, &repo_input.branch])
                .current_dir(&repo_path_str)
                .output()
                .map_err(|e| {
                    format!(
                        "Failed to create worktree in '{}': {}",
                        repo_input.rel_path, e
                    )
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!(
                    "git worktree add failed in '{}': {}",
                    repo_input.rel_path, stderr
                ));
            }

            results.push(ContextRepoResult {
                rel_path: repo_input.rel_path.clone(),
                worktree_path: normalize_path(&worktree_path),
                branch: repo_input.branch.clone(),
            });
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn delete_context(
    project_path: String,
    context_name: String,
    repos: Vec<ContextRepoInput>,
) -> Result<(), String> {
    let project_dir = Path::new(&project_path);

    for repo_input in &repos {
        let repo_path = project_dir.join(&repo_input.rel_path);
        let repo_path_str = repo_path.to_string_lossy().to_string();

        let worktree_dir = project_dir
            .join(".worktrees")
            .join(&context_name)
            .join(&repo_input.name);

        let worktree_path = worktree_dir.to_string_lossy().to_string();

        if worktree_dir.is_symlink() {
            std::fs::remove_file(&worktree_dir).map_err(|e| {
                format!(
                    "Failed to remove symlink '{}': {}",
                    worktree_path, e
                )
            })?;
        } else if worktree_dir.exists() {
            let output = std::process::Command::new("git")
                .args(["worktree", "remove", "--force", &worktree_path])
                .current_dir(&repo_path_str)
                .output()
                .map_err(|e| {
                    format!(
                        "Failed to remove worktree in '{}': {}",
                        repo_input.rel_path, e
                    )
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!(
                    "git worktree remove failed in '{}': {}",
                    repo_input.rel_path, stderr
                ));
            }

            let wt_path = Path::new(&worktree_path);
            if wt_path.exists() {
                std::fs::remove_dir_all(wt_path).map_err(|e| {
                    format!(
                        "Failed to delete worktree directory '{}': {}",
                        worktree_path, e
                    )
                })?;
            }
        }
    }

    let context_dir = project_dir.join(".worktrees").join(&context_name);
    if context_dir.exists() {
        let is_empty = context_dir
            .read_dir()
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if is_empty {
            std::fs::remove_dir(&context_dir).ok();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod normalize_remote_url {
        use super::*;

        #[test]
        fn converts_ssh_url_to_https() {
            let ssh_url = "git@github.com:user/repo.git";

            let result = normalize_remote_url(ssh_url);

            assert_eq!(result, "https://github.com/user/repo");
        }

        #[test]
        fn converts_ssh_url_without_git_suffix() {
            let ssh_url = "git@github.com:user/repo";

            let result = normalize_remote_url(ssh_url);

            assert_eq!(result, "https://github.com/user/repo");
        }

        #[test]
        fn strips_git_suffix_from_https_url() {
            let https_url = "https://github.com/user/repo.git";

            let result = normalize_remote_url(https_url);

            assert_eq!(result, "https://github.com/user/repo");
        }

        #[test]
        fn leaves_clean_https_url_unchanged() {
            let https_url = "https://github.com/user/repo";

            let result = normalize_remote_url(https_url);

            assert_eq!(result, "https://github.com/user/repo");
        }

        #[test]
        fn handles_gitlab_ssh_url() {
            let ssh_url = "git@gitlab.com:org/project.git";

            let result = normalize_remote_url(ssh_url);

            assert_eq!(result, "https://gitlab.com/org/project");
        }

        #[test]
        fn handles_nested_group_paths() {
            let ssh_url = "git@github.com:org/team/repo.git";

            let result = normalize_remote_url(ssh_url);

            assert_eq!(result, "https://github.com/org/team/repo");
        }
    }
}
