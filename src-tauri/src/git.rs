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
pub async fn get_worktree_status(path: String, base_branch: Option<String>) -> Result<WorktreeStatus, String> {
    let repo = Repository::open(&path).map_err(|e| e.to_string())?;

    let (changed_files, insertions, deletions) = match &base_branch {
        Some(base) => count_diff_against_base(&repo, base)?,
        None => {
            let files = count_changed_files(&repo)?;
            let (ins, del) = count_changed_lines(&repo);
            (files, ins, del)
        }
    };

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
    let mut total_insertions: usize = 0;
    let mut total_deletions: usize = 0;

    if let Ok(diff) = repo.diff_index_to_workdir(None, None) {
        if let Ok(stats) = diff.stats() {
            total_insertions += stats.insertions();
            total_deletions += stats.deletions();
        }
    }

    if let Ok(head) = repo.head() {
        if let Ok(tree) = head.peel_to_tree() {
            if let Ok(diff) = repo.diff_tree_to_index(Some(&tree), None, None) {
                if let Ok(stats) = diff.stats() {
                    total_insertions += stats.insertions();
                    total_deletions += stats.deletions();
                }
            }
        }
    }

    (total_insertions, total_deletions)
}

fn resolve_branch_commit<'a>(repo: &'a Repository, branch_name: &str) -> Result<git2::Commit<'a>, String> {
    if let Ok(branch) = repo.find_branch(branch_name, git2::BranchType::Local) {
        return branch.get().peel_to_commit().map_err(|e| e.to_string());
    }
    if let Ok(branch) = repo.find_branch(branch_name, git2::BranchType::Remote) {
        return branch.get().peel_to_commit().map_err(|e| e.to_string());
    }
    // Try as a full ref name (e.g. "origin/develop")
    let reference = repo
        .find_reference(&format!("refs/remotes/{}", branch_name))
        .map_err(|e| format!("Branch '{}' not found: {}", branch_name, e))?;
    reference.peel_to_commit().map_err(|e| e.to_string())
}

fn count_diff_against_base(repo: &Repository, base_branch: &str) -> Result<(FileChangeCounts, usize, usize), String> {
    let base_commit = resolve_branch_commit(repo, base_branch)?;

    let head_commit = repo.head().map_err(|e| e.to_string())?
        .peel_to_commit().map_err(|e| e.to_string())?;

    let merge_base_oid = repo
        .merge_base(base_commit.id(), head_commit.id())
        .map_err(|e| format!("Could not find merge base: {}", e))?;
    let merge_base_commit = repo.find_commit(merge_base_oid).map_err(|e| e.to_string())?;
    let merge_base_tree = merge_base_commit.tree().map_err(|e| e.to_string())?;

    let head_tree = head_commit.tree().map_err(|e| e.to_string())?;

    let diff = repo
        .diff_tree_to_tree(Some(&merge_base_tree), Some(&head_tree), None)
        .map_err(|e| e.to_string())?;

    let stats = diff.stats().map_err(|e| e.to_string())?;

    let mut counts = FileChangeCounts {
        added: 0,
        modified: 0,
        deleted: 0,
        renamed: 0,
    };

    for delta in diff.deltas() {
        match delta.status() {
            git2::Delta::Added => counts.added += 1,
            git2::Delta::Deleted => counts.deleted += 1,
            git2::Delta::Modified => counts.modified += 1,
            git2::Delta::Renamed => counts.renamed += 1,
            _ => {}
        }
    }

    Ok((counts, stats.insertions(), stats.deletions()))
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
    #[serde(default)]
    pub post_checkout_command: Option<String>,
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
    symlinks: Vec<String>,
    base_context_name: Option<String>,
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
            let symlink_target = match &base_context_name {
                Some(base_name) if base_name != "default" => {
                    project_dir
                        .join(".worktrees")
                        .join(base_name)
                        .join(&repo_input.name)
                }
                _ => repo_path.clone(),
            };

            if let Some(parent) = worktree_dir.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create parent directory: {}", e)
                })?;
            }

            #[cfg(unix)]
            std::os::unix::fs::symlink(&symlink_target, &worktree_dir).map_err(|e| {
                format!(
                    "Failed to create symlink for '{}': {}",
                    repo_input.name, e
                )
            })?;

            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(&symlink_target, &worktree_dir).map_err(|e| {
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

            // Prune stale worktree references before adding
            std::process::Command::new("git")
                .args(["worktree", "prune"])
                .current_dir(&repo_path_str)
                .output()
                .ok();

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

    // Run post-checkout commands
    for repo_input in &repos {
        if repo_input.linked {
            continue;
        }
        if let Some(ref cmd) = repo_input.post_checkout_command {
            if cmd.is_empty() {
                continue;
            }
            let worktree_dir = project_dir
                .join(".worktrees")
                .join(&context_name)
                .join(&repo_input.name);
            let worktree_path_str = worktree_dir.to_string_lossy().to_string();

            std::process::Command::new("sh")
                .args(["-c", cmd])
                .current_dir(&worktree_path_str)
                .output()
                .ok();
        }
    }

    // Create symlinks for configured paths
    let context_dir = project_dir.join(".worktrees").join(&context_name);
    for symlink_rel in &symlinks {
        let source = project_dir.join(symlink_rel);
        let link_name = Path::new(symlink_rel)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new(symlink_rel));
        let link_path = context_dir.join(link_name);

        if link_path.exists() || link_path.is_symlink() {
            continue;
        }

        if let Some(parent) = link_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!("Failed to create directory for symlink '{}': {}", symlink_rel, e)
            })?;
        }

        #[cfg(unix)]
        {
            if source.is_dir() {
                std::os::unix::fs::symlink(&source, &link_path).map_err(|e| {
                    format!("Failed to symlink '{}': {}", symlink_rel, e)
                })?;
            } else {
                std::os::unix::fs::symlink(&source, &link_path).map_err(|e| {
                    format!("Failed to symlink '{}': {}", symlink_rel, e)
                })?;
            }
        }

        #[cfg(windows)]
        {
            if source.is_dir() {
                std::os::windows::fs::symlink_dir(&source, &link_path).map_err(|e| {
                    format!("Failed to symlink '{}': {}", symlink_rel, e)
                })?;
            } else {
                std::os::windows::fs::symlink_file(&source, &link_path).map_err(|e| {
                    format!("Failed to symlink '{}': {}", symlink_rel, e)
                })?;
            }
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn delete_context(
    project_path: String,
    context_name: String,
    repos: Vec<ContextRepoInput>,
    symlinks: Vec<String>,
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

            // Prune stale worktree references
            std::process::Command::new("git")
                .args(["worktree", "prune"])
                .current_dir(&repo_path_str)
                .output()
                .ok();
        }
    }

    // Remove configured symlinks
    let context_dir = project_dir.join(".worktrees").join(&context_name);
    for symlink_rel in &symlinks {
        let link_name = Path::new(symlink_rel)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new(symlink_rel));
        let link_path = context_dir.join(link_name);

        if link_path.is_symlink() {
            std::fs::remove_file(&link_path).ok();
        }
    }

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

#[derive(Serialize)]
pub struct PullResult {
    pub path: String,
    pub pulled: bool,
    pub message: Option<String>,
}

fn has_upstream(repo: &Repository) -> bool {
    let Ok(head) = repo.head() else {
        return false;
    };

    if !head.is_branch() {
        return false;
    }

    let Some(branch_name) = head.shorthand() else {
        return false;
    };

    let Ok(branch) = repo.find_branch(branch_name, git2::BranchType::Local) else {
        return false;
    };

    branch.upstream().is_ok()
}

#[tauri::command]
pub async fn pull_worktree(path: String) -> Result<PullResult, String> {
    let repo = match Repository::open(&path) {
        Ok(repo) => repo,
        Err(e) => return Err(format!("Failed to open repository at '{}': {}", path, e)),
    };

    // Skip when there is no remote configured for this repository.
    if repo.find_remote("origin").is_err() {
        return Ok(PullResult {
            path: normalize_path(&path),
            pulled: false,
            message: Some("No remote configured".to_string()),
        });
    }

    if !has_upstream(&repo) {
        return Ok(PullResult {
            path: normalize_path(&path),
            pulled: false,
            message: Some("No upstream branch (not pushed yet)".to_string()),
        });
    }

    let output = std::process::Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(&path)
        .output()
        .map_err(|e| format!("Failed to run git pull: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git pull failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(PullResult {
        path: normalize_path(&path),
        pulled: true,
        message: Some(stdout.trim().to_string()),
    })
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
