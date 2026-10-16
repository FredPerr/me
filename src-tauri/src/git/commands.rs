//! Tauri commands of the Git bounded context. Names, inputs and outputs match
//! the commands the frontend already calls.
use std::path::Path;

use serde::Serialize;
use tauri::State;

use super::domain::branch_name::BranchName;
use super::domain::error::GitError;
use super::domain::paths::{RepositoryPath, WorktreePath};
use super::domain::ports::{Checkout, DiffStats, GitRepository, LocalBranch};
use super::{run_blocking, Git, PushListener};

const MAIN_CHECKOUT_NAME: &str = "default";

/// The kind of a git failure, as the frontend sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum GitErrorKind {
    NotARepository,
    InvalidBranchName,
    BranchNotFound,
    BranchAlreadyExists,
    BranchCheckedOut,
    NoRemote,
    NoUpstream,
    DetachedHead,
    NotFastForward,
    AuthenticationFailed,
    Network,
    Io,
    CommandFailed,
}

/// A failed git command on the wire. `message` is readable and may name local
/// branches and repeat git's output: it only describes the user's own repositories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GitCommandError {
    pub kind: GitErrorKind,
    pub message: String,
}

impl GitCommandError {
    /// Keeps the kind and replaces the message, e.g. to name the repository.
    pub fn with_message(self, message: impl Into<String>) -> Self {
        Self {
            kind: self.kind,
            message: message.into(),
        }
    }
}

impl From<GitError> for GitCommandError {
    fn from(error: GitError) -> Self {
        let kind = match &error {
            GitError::NotARepository { .. } => GitErrorKind::NotARepository,
            GitError::InvalidBranchName { .. } => GitErrorKind::InvalidBranchName,
            GitError::BranchNotFound { .. } => GitErrorKind::BranchNotFound,
            GitError::BranchAlreadyExists { .. } => GitErrorKind::BranchAlreadyExists,
            GitError::BranchCheckedOut { .. } => GitErrorKind::BranchCheckedOut,
            GitError::NoRemote => GitErrorKind::NoRemote,
            GitError::NoUpstream => GitErrorKind::NoUpstream,
            GitError::DetachedHead => GitErrorKind::DetachedHead,
            GitError::NotFastForward { .. } => GitErrorKind::NotFastForward,
            GitError::AuthenticationFailed { .. } => GitErrorKind::AuthenticationFailed,
            GitError::Network { .. } => GitErrorKind::Network,
            GitError::Io { .. } => GitErrorKind::Io,
            GitError::CommandFailed { .. } => GitErrorKind::CommandFailed,
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeView {
    pub name: String,
    pub path: String,
    pub branch: Option<String>,
    pub is_default: bool,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeStatusView {
    pub files_added: u32,
    pub files_modified: u32,
    pub files_deleted: u32,
    pub files_renamed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalBranchView {
    pub name: String,
    pub is_current: bool,
    pub is_checked_out: bool,
    pub has_upstream: bool,
    pub upstream_gone: bool,
    pub is_merged: bool,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PushResult {
    pub path: String,
    pub pushed: bool,
    pub message: Option<String>,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
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
    fn new(stats: DiffStats) -> Self {
        Self {
            files_added: saturate(stats.files_added),
            files_modified: saturate(stats.files_modified),
            files_deleted: saturate(stats.files_deleted),
            files_renamed: saturate(stats.files_renamed),
            insertions: saturate(stats.insertions),
            deletions: saturate(stats.deletions),
        }
    }
}

/// Counts beyond `u32` (over four billion) are shown as the maximum.
fn saturate(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
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

/// The text of a failure, keeping git's own output when there is any.
fn failure_detail(error: &GitError) -> String {
    error
        .git_output()
        .map(str::to_string)
        .unwrap_or_else(|| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_git_remote_url(
    git: State<'_, Git>,
    path: String,
) -> Result<Option<String>, GitCommandError> {
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
#[specta::specta]
pub async fn check_is_git_repository(
    git: State<'_, Git>,
    path: String,
) -> Result<bool, GitCommandError> {
    Ok(git.repositories().is_repository_root(Path::new(&path)))
}

#[tauri::command]
#[specta::specta]
pub async fn list_worktrees(
    git: State<'_, Git>,
    path: String,
) -> Result<Vec<WorktreeView>, GitCommandError> {
    let git = git.inner().clone();
    run_blocking(move || {
        let worktrees = open(&git, &path)?.worktrees()?;
        Ok(worktrees.into_iter().map(WorktreeView::from).collect())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_worktree_status(
    git: State<'_, Git>,
    path: String,
    base_branch: Option<String>,
) -> Result<WorktreeStatusView, GitCommandError> {
    let git = git.inner().clone();
    run_blocking(move || {
        let repository = open(&git, &path)?;
        let base = base_branch.as_deref().map(BranchName::parse).transpose()?;
        let stats = repository.diff_stats_against(base.as_ref())?;
        Ok(WorktreeStatusView::new(stats))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_worktree_diff(
    git: State<'_, Git>,
    path: String,
    base_branch: String,
) -> Result<String, GitCommandError> {
    let git = git.inner().clone();
    run_blocking(move || {
        let base = BranchName::parse(&base_branch)?;
        Ok(open(&git, &path)?.diff_against(&base)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_branches(
    git: State<'_, Git>,
    path: String,
) -> Result<Vec<String>, GitCommandError> {
    let git = git.inner().clone();
    run_blocking(move || {
        let names = open(&git, &path)?.branch_names()?;
        Ok(names.into_iter().map(|name| name.to_string()).collect())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_local_branches(
    git: State<'_, Git>,
    path: String,
) -> Result<Vec<LocalBranchView>, GitCommandError> {
    let git = git.inner().clone();
    run_blocking(move || {
        let branches = open(&git, &path)?.branches()?;
        Ok(branches.into_iter().map(LocalBranchView::from).collect())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_local_branch(
    git: State<'_, Git>,
    path: String,
    branch: String,
    force: bool,
) -> Result<(), GitCommandError> {
    let git = git.inner().clone();
    run_blocking(move || {
        let name = BranchName::parse(&branch)?;
        open(&git, &path)?
            .delete_branch(&name, force)
            .map_err(|error| {
                let message = format!(
                    "Failed to delete branch '{branch}': {}",
                    failure_detail(&error)
                );
                GitCommandError::from(error).with_message(message)
            })
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn prune_worktrees(git: State<'_, Git>, path: String) -> Result<(), GitCommandError> {
    let git = git.inner().clone();
    run_blocking(move || Ok(open(&git, &path)?.prune_worktrees()?)).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_worktree(
    git: State<'_, Git>,
    repo_path: String,
    worktree_path: String,
) -> Result<(), GitCommandError> {
    let git = git.inner().clone();
    run_blocking(move || {
        Ok(open(&git, &repo_path)?.remove_worktree(&WorktreePath::new(&worktree_path))?)
    })
    .await
}

/// Pushes the checkout's current branch to its upstream.
#[tauri::command]
#[specta::specta]
pub async fn push_worktree(
    git: State<'_, Git>,
    push_listener: State<'_, PushListener>,
    path: String,
) -> Result<PushResult, GitCommandError> {
    let git = git.inner().clone();
    let push_listener = push_listener.inner().clone();
    run_blocking(move || push_checkout(&git, &push_listener, &path)).await
}

/// Once the branch is pushed, `push_listener` hears about it.
fn push_checkout(
    git: &Git,
    push_listener: &PushListener,
    path: &str,
) -> Result<PushResult, GitCommandError> {
    let display_path = RepositoryPath::new(path).display();
    let repository = open(git, path)?;
    match repository.push() {
        Ok(report) => {
            if let (Some(remote), Some(branch)) =
                (repository.remote_url(), repository.current_branch())
            {
                push_listener.0.branch_pushed(&remote, &branch);
            }
            Ok(PushResult {
                path: display_path,
                pushed: true,
                message: Some(report),
            })
        }
        Err(skipped @ (GitError::NoRemote | GitError::NoUpstream)) => Ok(PushResult {
            path: display_path,
            pushed: false,
            message: Some(skipped.to_string()),
        }),
        Err(error) => Err(error.into()),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn pull_worktree(
    git: State<'_, Git>,
    path: String,
) -> Result<PullResult, GitCommandError> {
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
#[specta::specta]
pub async fn fetch_pr_branch(
    git: State<'_, Git>,
    path: String,
    branch: String,
) -> Result<(), GitCommandError> {
    let git = git.inner().clone();
    run_blocking(move || {
        let name = BranchName::parse(&branch)?;
        open(&git, &path)?.fetch_branch(&name).map_err(|error| {
            let message = match &error {
                GitError::NoRemote => "No 'origin' remote configured".to_string(),
                other => format!(
                    "Could not fetch branch '{branch}' from origin: {}",
                    failure_detail(other)
                ),
            };
            GitCommandError::from(error).with_message(message)
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(error: GitError) -> GitCommandError {
        GitCommandError::from(error)
    }

    #[test]
    fn every_git_error_maps_to_its_kind() {
        let output = || "fatal: details".to_string();
        let operation = || "push".to_string();
        let cases = [
            (
                GitError::NotARepository {
                    path: "/tmp/x".into(),
                },
                GitErrorKind::NotARepository,
            ),
            (
                GitError::InvalidBranchName {
                    name: "a..b".into(),
                    reason: "contains \"..\"",
                },
                GitErrorKind::InvalidBranchName,
            ),
            (
                GitError::BranchNotFound {
                    name: "main".into(),
                },
                GitErrorKind::BranchNotFound,
            ),
            (
                GitError::BranchAlreadyExists {
                    name: "main".into(),
                },
                GitErrorKind::BranchAlreadyExists,
            ),
            (
                GitError::BranchCheckedOut {
                    name: "main".into(),
                    worktree: "/w".into(),
                },
                GitErrorKind::BranchCheckedOut,
            ),
            (GitError::NoRemote, GitErrorKind::NoRemote),
            (GitError::NoUpstream, GitErrorKind::NoUpstream),
            (GitError::DetachedHead, GitErrorKind::DetachedHead),
            (
                GitError::NotFastForward {
                    operation: operation(),
                    output: output(),
                },
                GitErrorKind::NotFastForward,
            ),
            (
                GitError::AuthenticationFailed {
                    operation: operation(),
                    output: output(),
                },
                GitErrorKind::AuthenticationFailed,
            ),
            (
                GitError::Network {
                    operation: operation(),
                    output: output(),
                },
                GitErrorKind::Network,
            ),
            (
                GitError::Io {
                    action: "read".into(),
                    detail: "denied".into(),
                },
                GitErrorKind::Io,
            ),
            (
                GitError::CommandFailed {
                    operation: operation(),
                    output: output(),
                },
                GitErrorKind::CommandFailed,
            ),
        ];
        for (error, kind) in cases {
            let message = error.to_string();
            let wire_error = wire(error);
            assert_eq!(wire_error.kind, kind);
            assert_eq!(wire_error.message, message);
        }
    }

    #[test]
    fn keeps_branch_names_and_git_output_in_the_message() {
        let wire_error = wire(GitError::CommandFailed {
            operation: "push".into(),
            output: "rejected feature/login".into(),
        });
        assert_eq!(
            wire_error.message,
            "git push failed: rejected feature/login"
        );
    }

    #[test]
    fn serializes_kind_in_camel_case() {
        let json = serde_json::to_value(wire(GitError::NoUpstream)).unwrap();
        assert_eq!(json["kind"], "noUpstream");
        assert_eq!(json["message"], "No upstream branch (not pushed yet)");
    }

    mod push_checkout {
        use std::sync::{Arc, Mutex};

        use super::*;
        use crate::git::domain::ports::{BranchPushListener, GitRepositories};
        use crate::git::domain::remote_url::RemoteUrl;
        use crate::git::infra::in_memory::InMemoryGitRepositories;

        const API: &str = "/work/api";
        const LOGIN_CHECKOUT: &str = "/work/.worktrees/login/api";
        const MAIN: &str = "main";
        const LOGIN_BRANCH: &str = "feature/login";
        const ORIGIN_URL: &str = "git@github.com:acme/api.git";

        #[derive(Default)]
        struct RecordingPushListener {
            pushed: Mutex<Vec<(String, String)>>,
        }

        impl BranchPushListener for RecordingPushListener {
            fn branch_pushed(&self, remote: &RemoteUrl, branch: &BranchName) {
                self.pushed
                    .lock()
                    .unwrap()
                    .push((remote.as_str().to_string(), branch.to_string()));
            }
        }

        fn branch(raw: &str) -> BranchName {
            BranchName::parse(raw).unwrap()
        }

        struct Fixture {
            repositories: Arc<InMemoryGitRepositories>,
            listener: Arc<RecordingPushListener>,
        }

        impl Fixture {
            /// api with a remote, and the login branch checked out at its worktree.
            fn new() -> Self {
                let repositories = Arc::new(InMemoryGitRepositories::new());
                let api = RepositoryPath::new(API);
                repositories.add_repository(&api, &branch(MAIN));
                repositories.set_remote(&api, RemoteUrl::new(ORIGIN_URL));
                let repository = repositories.open(&api).unwrap();
                repository
                    .create_branch(&branch(LOGIN_BRANCH), &branch(MAIN))
                    .unwrap();
                repository
                    .add_worktree(&WorktreePath::new(LOGIN_CHECKOUT), &branch(LOGIN_BRANCH))
                    .unwrap();
                Self {
                    repositories,
                    listener: Arc::new(RecordingPushListener::default()),
                }
            }

            fn publish_login(&self) {
                let api = RepositoryPath::new(API);
                self.repositories.publish(&api, &branch(LOGIN_BRANCH));
                self.repositories.commit(&api, &branch(LOGIN_BRANCH));
            }

            fn push(&self) -> Result<PushResult, GitCommandError> {
                let git = Git(self.repositories.clone());
                let listener = PushListener::new(self.listener.clone());
                push_checkout(&git, &listener, LOGIN_CHECKOUT)
            }

            fn heard(&self) -> Vec<(String, String)> {
                self.listener.pushed.lock().unwrap().clone()
            }
        }

        #[test]
        fn tells_the_listener_about_the_pushed_branch() {
            let fixture = Fixture::new();
            fixture.publish_login();

            let pushed = fixture.push().unwrap();

            assert!(pushed.pushed);
            assert_eq!(
                fixture.heard(),
                vec![(ORIGIN_URL.to_string(), LOGIN_BRANCH.to_string())]
            );
        }

        #[test]
        fn a_branch_without_upstream_is_published_and_heard() {
            let fixture = Fixture::new();

            let pushed = fixture.push().unwrap();

            assert!(pushed.pushed);
            assert_eq!(
                fixture.heard(),
                vec![(ORIGIN_URL.to_string(), LOGIN_BRANCH.to_string())]
            );
        }
    }

    #[test]
    fn worktree_names_follow_the_context_layout() {
        assert_eq!(
            readable_worktree_name("/work/.worktrees/staging/backend"),
            "staging/backend"
        );
        assert_eq!(readable_worktree_name("backend"), "backend");
    }
}
