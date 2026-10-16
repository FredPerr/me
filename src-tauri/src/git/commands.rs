//! Tauri commands of the Git bounded context. Names, inputs and outputs match
//! the commands the frontend already calls.
use std::path::Path;

use serde::Serialize;
use tauri::State;

use super::domain::branch_name::BranchName;
use super::domain::error::GitError;
use super::domain::paths::{RepositoryPath, WorktreePath};
use super::domain::ports::{Checkout, DiffStats, GitRepository, LocalBranch};
use super::{run_blocking, Git};

const MAIN_CHECKOUT_NAME: &str = "default";

#[derive(Serialize)]
pub struct WorktreeView {
    pub name: String,
    pub path: String,
    pub branch: Option<String>,
    pub is_default: bool,
}

#[derive(Serialize)]
pub struct WorktreeStatusView {
    pub files_added: usize,
    pub files_modified: usize,
    pub files_deleted: usize,
    pub files_renamed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub pr_url: Option<String>,
}

#[derive(Serialize)]
pub struct LocalBranchView {
    pub name: String,
    pub is_current: bool,
    pub is_checked_out: bool,
    pub has_upstream: bool,
    pub upstream_gone: bool,
    pub is_merged: bool,
}

#[derive(Serialize)]
pub struct PushResult {
    pub path: String,
    pub pushed: bool,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct PullResult {
    pub path: String,
    pub pulled: bool,
    pub message: Option<String>,
}

impl From<LocalBranch> for LocalBranchView {
    fn from(branch: LocalBranch) -> Self {
        Self {
            name: branch.name.to_string(),
            is_current: branch.is_current,
            is_checked_out: branch.is_checked_out,
            has_upstream: branch.has_upstream,
            upstream_gone: branch.upstream_gone,
            is_merged: branch.is_merged,
        }
    }
}

impl From<Checkout> for WorktreeView {
    fn from(checkout: Checkout) -> Self {
        let path = checkout.path.display();
        Self {
            name: if checkout.is_main {
                MAIN_CHECKOUT_NAME.to_string()
            } else {
                readable_worktree_name(&path)
            },
            path,
            branch: checkout.branch.map(|branch| branch.to_string()),
            is_default: checkout.is_main,
        }
    }
}

impl WorktreeStatusView {
    fn new(stats: DiffStats, pr_url: Option<String>) -> Self {
        Self {
            files_added: stats.files_added,
            files_modified: stats.files_modified,
            files_deleted: stats.files_deleted,
            files_renamed: stats.files_renamed,
            insertions: stats.insertions,
            deletions: stats.deletions,
            pr_url,
        }
    }
}

/// git's internal worktree name is not meaningful; the app's layout
/// `.worktrees/<context>/<repository>` gives "<context>/<repository>".
fn readable_worktree_name(worktree_path: &str) -> String {
    let path = Path::new(worktree_path);
    let folder_name = |path: &Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
    };
    match (path.parent().and_then(folder_name), folder_name(path)) {
        (Some(context), Some(repository)) => format!("{context}/{repository}"),
        (None, Some(repository)) => repository,
        _ => worktree_path.to_string(),
    }
}

fn open(git: &Git, path: &str) -> Result<Box<dyn GitRepository>, GitError> {
    git.repositories().open(&RepositoryPath::new(path))
}

/// The pull request compare page for the current branch, on GitHub only.
fn github_compare_url(repository: &dyn GitRepository) -> Option<String> {
    let branch = repository.current_branch()?;
    repository.remote_url()?.github_compare_url(&branch)
}

/// The text of a failure, keeping git's own output when there is any.
fn failure_detail(error: &GitError) -> String {
    error
        .git_output()
        .map(str::to_string)
        .unwrap_or_else(|| error.to_string())
}

#[tauri::command]
pub async fn get_git_remote_url(
    git: State<'_, Git>,
    path: String,
) -> Result<Option<String>, String> {
    let git = git.inner().clone();
    run_blocking(move || {
        if !git.repositories().is_repository_root(Path::new(&path)) {
            return Ok(None);
        }
        Ok(open(&git, &path)
            .ok()
            .and_then(|repository| repository.remote_url())
            .map(|remote| remote.web_url()))
    })
    .await
}

#[tauri::command]
pub async fn check_is_git_repository(git: State<'_, Git>, path: String) -> Result<bool, String> {
    Ok(git.repositories().is_repository_root(Path::new(&path)))
}

#[tauri::command]
pub async fn list_worktrees(
    git: State<'_, Git>,
    path: String,
) -> Result<Vec<WorktreeView>, String> {
    let git = git.inner().clone();
    run_blocking(move || {
        let worktrees = open(&git, &path)?.worktrees()?;
        Ok(worktrees.into_iter().map(WorktreeView::from).collect())
    })
    .await
}

#[tauri::command]
pub async fn get_worktree_status(
    git: State<'_, Git>,
    path: String,
    base_branch: Option<String>,
) -> Result<WorktreeStatusView, String> {
    let git = git.inner().clone();
    run_blocking(move || {
        let repository = open(&git, &path)?;
        let base = base_branch.as_deref().map(BranchName::parse).transpose()?;
        let stats = repository.diff_stats_against(base.as_ref())?;
        Ok(WorktreeStatusView::new(
            stats,
            github_compare_url(repository.as_ref()),
        ))
    })
    .await
}

#[tauri::command]
pub async fn get_worktree_diff(
    git: State<'_, Git>,
    path: String,
    base_branch: String,
) -> Result<String, String> {
    let git = git.inner().clone();
    run_blocking(move || {
        let base = BranchName::parse(&base_branch)?;
        Ok(open(&git, &path)?.diff_against(&base)?)
    })
    .await
}

#[tauri::command]
pub async fn list_branches(git: State<'_, Git>, path: String) -> Result<Vec<String>, String> {
    let git = git.inner().clone();
    run_blocking(move || {
        let names = open(&git, &path)?.branch_names()?;
        Ok(names.into_iter().map(|name| name.to_string()).collect())
    })
    .await
}

#[tauri::command]
pub async fn list_local_branches(
    git: State<'_, Git>,
    path: String,
) -> Result<Vec<LocalBranchView>, String> {
    let git = git.inner().clone();
    run_blocking(move || {
        let branches = open(&git, &path)?.branches()?;
        Ok(branches.into_iter().map(LocalBranchView::from).collect())
    })
    .await
}

#[tauri::command]
pub async fn delete_local_branch(
    git: State<'_, Git>,
    path: String,
    branch: String,
    force: bool,
) -> Result<(), String> {
    let git = git.inner().clone();
    run_blocking(move || {
        let name = BranchName::parse(&branch)?;
        open(&git, &path)?
            .delete_branch(&name, force)
            .map_err(|error| {
                format!(
                    "Failed to delete branch '{branch}': {}",
                    failure_detail(&error)
                )
            })
    })
    .await
}

#[tauri::command]
pub async fn prune_worktrees(git: State<'_, Git>, path: String) -> Result<(), String> {
    let git = git.inner().clone();
    run_blocking(move || Ok(open(&git, &path)?.prune_worktrees()?)).await
}

#[tauri::command]
pub async fn delete_worktree(
    git: State<'_, Git>,
    repo_path: String,
    worktree_path: String,
) -> Result<(), String> {
    let git = git.inner().clone();
    run_blocking(move || {
        Ok(open(&git, &repo_path)?.remove_worktree(&WorktreePath::new(&worktree_path))?)
    })
    .await
}

#[tauri::command]
pub async fn push_worktree(git: State<'_, Git>, path: String) -> Result<PushResult, String> {
    let git = git.inner().clone();
    run_blocking(move || {
        let display_path = RepositoryPath::new(&path).display();
        match open(&git, &path)?.push() {
            Ok(report) => Ok(PushResult {
                path: display_path,
                pushed: true,
                message: Some(report),
            }),
            Err(skipped @ (GitError::NoRemote | GitError::NoUpstream)) => Ok(PushResult {
                path: display_path,
                pushed: false,
                message: Some(skipped.to_string()),
            }),
            Err(error) => Err(error.into()),
        }
    })
    .await
}

#[tauri::command]
pub async fn pull_worktree(git: State<'_, Git>, path: String) -> Result<PullResult, String> {
    let git = git.inner().clone();
    run_blocking(move || {
        let display_path = RepositoryPath::new(&path).display();
        match open(&git, &path)?.pull() {
            Ok(report) => Ok(PullResult {
                path: display_path,
                pulled: true,
                message: Some(report),
            }),
            Err(skipped @ (GitError::NoRemote | GitError::NoUpstream)) => Ok(PullResult {
                path: display_path,
                pulled: false,
                message: Some(skipped.to_string()),
            }),
            Err(error) => Err(error.into()),
        }
    })
    .await
}

#[tauri::command]
pub async fn fetch_pr_branch(
    git: State<'_, Git>,
    path: String,
    branch: String,
) -> Result<(), String> {
    let git = git.inner().clone();
    run_blocking(move || {
        let name = BranchName::parse(&branch)?;
        open(&git, &path)?
            .fetch_branch(&name)
            .map_err(|error| match error {
                GitError::NoRemote => "No 'origin' remote configured".to_string(),
                other => format!(
                    "Could not fetch branch '{branch}' from origin: {}",
                    failure_detail(&other)
                ),
            })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_names_follow_the_context_layout() {
        assert_eq!(
            readable_worktree_name("/work/.worktrees/staging/backend"),
            "staging/backend"
        );
        assert_eq!(readable_worktree_name("backend"), "backend");
    }
}
