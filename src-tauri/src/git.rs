use git2::Repository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

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
                    // git's internal worktree name (e.g. "backend1") is not
                    // meaningful; derive a readable name from the path layout
                    // ".worktrees/<context>/<repo>" instead.
                    name: worktree_display_name(&wt_path),
                    path: normalize_path(&wt_path),
                    branch,
                    is_default: false,
                });
            }
        }
    }

    Ok(worktrees)
}

/// Builds a readable worktree name from its path. For the app's layout
/// ".worktrees/<context>/<repo>" this yields "<context>/<repo>" (e.g.
/// "staging/backend"), which is unambiguous unlike git's internal name.
/// Falls back to the final path segment when a parent is unavailable.
fn worktree_display_name(worktree_path: &str) -> String {
    let path = Path::new(worktree_path);
    let leaf = path.file_name().map(|s| s.to_string_lossy().to_string());
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_string());

    match (parent, leaf) {
        (Some(parent), Some(leaf)) => format!("{}/{}", parent, leaf),
        (None, Some(leaf)) => leaf,
        _ => worktree_path.to_string(),
    }
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
pub async fn get_worktree_status(
    path: String,
    base_branch: Option<String>,
) -> Result<WorktreeStatus, String> {
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

fn resolve_branch_commit<'a>(
    repo: &'a Repository,
    branch_name: &str,
) -> Result<git2::Commit<'a>, String> {
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

fn count_diff_against_base(
    repo: &Repository,
    base_branch: &str,
) -> Result<(FileChangeCounts, usize, usize), String> {
    let base_commit = resolve_branch_commit(repo, base_branch)?;

    let head_commit = repo
        .head()
        .map_err(|e| e.to_string())?
        .peel_to_commit()
        .map_err(|e| e.to_string())?;

    let merge_base_oid = repo
        .merge_base(base_commit.id(), head_commit.id())
        .map_err(|e| format!("Could not find merge base: {}", e))?;
    let merge_base_commit = repo
        .find_commit(merge_base_oid)
        .map_err(|e| e.to_string())?;
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

#[derive(Serialize)]
pub struct LocalBranch {
    pub name: String,
    /// The branch is HEAD of the main working tree.
    pub is_current: bool,
    /// The branch is checked out in the main tree or any linked worktree and so
    /// cannot be deleted without removing that worktree first.
    pub is_checked_out: bool,
    /// The branch has a configured upstream tracking branch.
    pub has_upstream: bool,
    /// The branch had an upstream but its remote-tracking ref no longer exists
    /// (the remote branch was deleted). A strong signal the branch is dead.
    pub upstream_gone: bool,
    /// The branch is merged into the repository HEAD.
    pub is_merged: bool,
}

fn checked_out_branches(repo: &Repository) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();

    if let Some(branch) = get_head_branch(repo) {
        names.insert(branch);
    }

    if let Ok(worktree_names) = repo.worktrees() {
        for i in 0..worktree_names.len() {
            let Some(name) = worktree_names.get(i) else {
                continue;
            };
            if let Ok(wt) = repo.find_worktree(name) {
                let wt_path = wt.path().to_string_lossy().to_string();
                if let Some(branch) = get_worktree_branch(repo, &wt_path) {
                    names.insert(branch);
                }
            }
        }
    }

    names
}

#[tauri::command]
pub async fn list_local_branches(path: String) -> Result<Vec<LocalBranch>, String> {
    let repo = Repository::open(&path).map_err(|e| e.to_string())?;

    let checked_out = checked_out_branches(&repo);
    let current = get_head_branch(&repo);

    let head_commit = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());

    let branches = repo
        .branches(Some(git2::BranchType::Local))
        .map_err(|e| e.to_string())?;

    let mut local_branches: Vec<LocalBranch> = Vec::new();
    for branch in branches {
        let (branch, _) = branch.map_err(|e| e.to_string())?;
        let Some(name) = branch.name().map_err(|e| e.to_string())?.map(String::from) else {
            continue;
        };

        let upstream = branch.upstream();
        let has_upstream = upstream.is_ok();
        // git2 returns NotFound for a configured-but-missing upstream ref.
        let upstream_gone = matches!(&upstream, Err(e) if e.code() == git2::ErrorCode::NotFound)
            && branch_has_upstream_config(&repo, &name);

        let is_merged = match (&head_commit, branch.get().peel_to_commit().ok()) {
            (Some(head), Some(branch_commit)) => repo
                .graph_descendant_of(head.id(), branch_commit.id())
                .unwrap_or(false)
                || head.id() == branch_commit.id(),
            _ => false,
        };

        local_branches.push(LocalBranch {
            is_current: current.as_deref() == Some(name.as_str()),
            is_checked_out: checked_out.contains(&name),
            has_upstream,
            upstream_gone,
            is_merged,
            name,
        });
    }

    local_branches.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(local_branches)
}

fn branch_has_upstream_config(repo: &Repository, branch: &str) -> bool {
    repo.config()
        .and_then(|cfg| cfg.get_string(&format!("branch.{}.merge", branch)))
        .is_ok()
}

#[tauri::command]
pub async fn delete_local_branch(
    path: String,
    branch: String,
    force: bool,
) -> Result<(), String> {
    let delete_flag = if force { "-D" } else { "-d" };

    let output = std::process::Command::new("git")
        .args(["branch", delete_flag, &branch])
        .current_dir(&path)
        .output()
        .map_err(|e| format!("Failed to run git branch: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to delete branch '{}': {}", branch, stderr.trim()));
    }

    Ok(())
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
pub async fn prune_worktrees(path: String) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .args(["worktree", "prune"])
        .current_dir(&path)
        .output()
        .map_err(|e| format!("Failed to run git worktree prune: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git worktree prune failed: {}", stderr.trim()));
    }

    Ok(())
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

/// Returns the worktree path where `branch` is currently checked out, if any.
/// Parses `git worktree list --porcelain`, whose records pair a `worktree <path>`
/// line with a `branch refs/heads/<name>` line.
fn worktree_path_for_branch(repo_path: &str, branch: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let target_ref = format!("refs/heads/{}", branch);
    let mut current_path: Option<String> = None;

    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(path.to_string());
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            if branch_ref == target_ref {
                return current_path;
            }
        }
    }

    None
}

/// Removes a filesystem entry at `path` if it exists, handling symlinks, files,
/// and directories. Uses `symlink_metadata` so that a symlink is removed as a
/// link rather than followed to its target.
fn remove_existing_path(path: &Path) -> std::io::Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };

    if metadata.file_type().is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
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
                Some(base_name) if base_name != "default" => project_dir
                    .join(".worktrees")
                    .join(base_name)
                    .join(&repo_input.name),
                _ => repo_path.clone(),
            };

            if let Some(parent) = worktree_dir.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }

            // Remove any leftover entry (e.g. a stale symlink from a previous
            // attempt) so re-creating the context is idempotent.
            remove_existing_path(&worktree_dir).map_err(|e| {
                format!("Failed to clear existing path for '{}': {}", repo_input.name, e)
            })?;

            #[cfg(unix)]
            std::os::unix::fs::symlink(&symlink_target, &worktree_dir).map_err(|e| {
                format!("Failed to create symlink for '{}': {}", repo_input.name, e)
            })?;

            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(&symlink_target, &worktree_dir).map_err(|e| {
                format!("Failed to create symlink for '{}': {}", repo_input.name, e)
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

            // A branch can be checked out in only one worktree. If it is already
            // checked out somewhere, reuse that worktree (point the context at
            // its existing location) instead of failing on `git worktree add`.
            let existing_worktree =
                worktree_path_for_branch(&repo_path_str, &repo_input.branch);

            let effective_worktree_path = match existing_worktree {
                Some(existing) => normalize_path(&existing),
                None => {
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

                    normalize_path(&worktree_path)
                }
            };

            results.push(ContextRepoResult {
                rel_path: repo_input.rel_path.clone(),
                worktree_path: effective_worktree_path,
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
                format!(
                    "Failed to create directory for symlink '{}': {}",
                    symlink_rel, e
                )
            })?;
        }

        #[cfg(unix)]
        {
            if source.is_dir() {
                std::os::unix::fs::symlink(&source, &link_path)
                    .map_err(|e| format!("Failed to symlink '{}': {}", symlink_rel, e))?;
            } else {
                std::os::unix::fs::symlink(&source, &link_path)
                    .map_err(|e| format!("Failed to symlink '{}': {}", symlink_rel, e))?;
            }
        }

        #[cfg(windows)]
        {
            if source.is_dir() {
                std::os::windows::fs::symlink_dir(&source, &link_path)
                    .map_err(|e| format!("Failed to symlink '{}': {}", symlink_rel, e))?;
            } else {
                std::os::windows::fs::symlink_file(&source, &link_path)
                    .map_err(|e| format!("Failed to symlink '{}': {}", symlink_rel, e))?;
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
            std::fs::remove_file(&worktree_dir)
                .map_err(|e| format!("Failed to remove symlink '{}': {}", worktree_path, e))?;
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

/// The upstream a worktree branch tracks, split into the remote name and the
/// branch name on that remote (e.g. "origin" and "main").
struct Upstream {
    remote: String,
    remote_branch: String,
}

fn resolve_upstream(repo: &Repository) -> Option<Upstream> {
    let head = repo.head().ok()?;
    if !head.is_branch() {
        return None;
    }

    let branch_name = head.shorthand()?;
    let branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .ok()?;
    let upstream = branch.upstream().ok()?;

    // upstream shorthand looks like "origin/main"; the remote is resolved via
    // the branch's configured remote so slashes in either name stay correct.
    let remote = repo
        .branch_upstream_remote(&format!("refs/heads/{}", branch_name))
        .ok()?;
    let remote = remote.as_str()?.to_string();

    let upstream_ref = upstream.get().name()?;
    let prefix = format!("refs/remotes/{}/", remote);
    let remote_branch = upstream_ref.strip_prefix(&prefix)?.to_string();

    Some(Upstream {
        remote,
        remote_branch,
    })
}

/// Build the `git -c http.<host>.extraheader=...` arguments that authenticate a
/// network operation against the given remote using a stored provider token.
/// Returns an empty vec when the remote is unknown, uses SSH, or no token is
/// stored — in which case git falls back to its ambient credentials.
fn remote_auth_args(repo: &Repository, remote_name: &str) -> Vec<String> {
    let Ok(remote) = repo.find_remote(remote_name) else {
        return Vec::new();
    };
    let Some(remote_url) = remote.url() else {
        return Vec::new();
    };
    crate::git_provider::git_auth::auth_args_for_remote(remote_url)
}

#[derive(Serialize)]
pub struct PushResult {
    pub path: String,
    pub pushed: bool,
    pub message: Option<String>,
}

/// Push the current branch of the worktree at `path` to its upstream, injecting
/// the stored provider access token for authentication when the remote is a
/// supported HTTPS provider. Requires the branch to already track an upstream.
#[tauri::command]
pub async fn push_worktree(path: String) -> Result<PushResult, String> {
    let repo = Repository::open(&path)
        .map_err(|e| format!("Failed to open repository at '{}': {}", path, e))?;

    if repo.find_remote("origin").is_err() {
        return Ok(PushResult {
            path: normalize_path(&path),
            pushed: false,
            message: Some("No remote configured".to_string()),
        });
    }

    let Some(upstream) = resolve_upstream(&repo) else {
        return Ok(PushResult {
            path: normalize_path(&path),
            pushed: false,
            message: Some("No upstream branch (not pushed yet)".to_string()),
        });
    };

    let Some(local_branch) = get_head_branch(&repo) else {
        return Err("Cannot push a detached HEAD".to_string());
    };

    let auth_args = remote_auth_args(&repo, &upstream.remote);
    let refspec = format!("{}:{}", local_branch, upstream.remote_branch);

    let push = std::process::Command::new("git")
        .args(&auth_args)
        .args(["push", &upstream.remote, &refspec])
        .current_dir(&path)
        .output()
        .map_err(|e| format!("Failed to run git push: {}", e))?;

    if !push.status.success() {
        let stderr = String::from_utf8_lossy(&push.stderr);
        return Err(format!("git push failed: {}", stderr.trim()));
    }

    // git writes push progress/status to stderr even on success.
    let stderr = String::from_utf8_lossy(&push.stderr);
    Ok(PushResult {
        path: normalize_path(&path),
        pushed: true,
        message: Some(stderr.trim().to_string()),
    })
}

/// Worktrees that belong to the same repository share a single `$GIT_DIR`
/// (and therefore a single `FETCH_HEAD`). Running `git fetch` for two of them
/// at once interleaves writes to that file and corrupts it, which surfaces as
/// `fatal: Cannot fast-forward to multiple branches`. We serialize fetches per
/// common git directory with a lock keyed by that directory.
fn repo_lock(common_dir: &Path) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let registry = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));

    let mut registry = registry.lock().expect("repo lock registry poisoned");
    registry
        .entry(common_dir.to_path_buf())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// Resolves the common git directory shared by all worktrees of a repository.
/// Linked worktrees each have their own `$GIT_DIR` but share `--git-common-dir`,
/// so this is what identifies "the same repository" for locking purposes.
fn common_git_dir(path: &str) -> PathBuf {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !dir.is_empty() {
                let dir = PathBuf::from(dir);
                return std::fs::canonicalize(&dir).unwrap_or(dir);
            }
        }
    }

    // Fallback: treat the path itself as the key so we still serialize per path.
    PathBuf::from(path)
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

    let Some(upstream) = resolve_upstream(&repo) else {
        return Ok(PullResult {
            path: normalize_path(&path),
            pulled: false,
            message: Some("No upstream branch (not pushed yet)".to_string()),
        });
    };

    let lock = repo_lock(&common_git_dir(&path));
    let _guard = lock.lock().expect("repo lock poisoned");

    let auth_args = remote_auth_args(&repo, &upstream.remote);

    // Fetch only the tracked upstream branch so FETCH_HEAD holds exactly one
    // "for-merge" entry, then fast-forward the worktree to it. This avoids the
    // multi-branch fast-forward error that a bare `git pull` hits when the
    // shared FETCH_HEAD has been written by a concurrent fetch.
    let fetch = std::process::Command::new("git")
        .args(&auth_args)
        .args(["fetch", &upstream.remote, &upstream.remote_branch])
        .current_dir(&path)
        .output()
        .map_err(|e| format!("Failed to run git fetch: {}", e))?;

    if !fetch.status.success() {
        let stderr = String::from_utf8_lossy(&fetch.stderr);
        return Err(format!("git fetch failed: {}", stderr.trim()));
    }

    let merge = std::process::Command::new("git")
        .args(["merge", "--ff-only", "FETCH_HEAD"])
        .current_dir(&path)
        .output()
        .map_err(|e| format!("Failed to run git merge: {}", e))?;

    if !merge.status.success() {
        let stderr = String::from_utf8_lossy(&merge.stderr);
        return Err(format!("git pull failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&merge.stdout);
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
