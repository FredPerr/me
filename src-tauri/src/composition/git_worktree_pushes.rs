//! `git_hosting`'s `WorktreePushes`, pushed by `git` with the credentials it
//! was built with. git's own output is logged, never handed to `git_hosting`.

use std::sync::Arc;

use async_trait::async_trait;

use crate::git::domain::error::GitError;
use crate::git::domain::paths::RepositoryPath;
use crate::git::domain::ports::GitRepositories;
use crate::git::run_blocking;
use crate::git_hosting::domain::error::PushFailureReason;
use crate::git_hosting::domain::ports::{PushFailure, WorktreePushes};

pub struct GitWorktreePushes {
    git: Arc<dyn GitRepositories>,
}

impl GitWorktreePushes {
    pub fn new(git: Arc<dyn GitRepositories>) -> Self {
        Self { git }
    }
}

#[async_trait]
impl WorktreePushes for GitWorktreePushes {
    async fn push(&self, worktree_path: &str) -> Result<(), PushFailure> {
        let git = Arc::clone(&self.git);
        let path = RepositoryPath::new(worktree_path);
        let pushed: Result<String, GitError> = run_blocking(move || git.open(&path)?.push()).await;
        pushed.map(|_| ()).map_err(push_failure)
    }
}

/// No remote or upstream means there is nothing to push to, as `push_worktree`
/// reports it.
fn push_failure(error: GitError) -> PushFailure {
    match error {
        GitError::NoRemote | GitError::NoUpstream => PushFailure::Skipped,
        failed => {
            eprintln!("[git_hosting] push failed: {failed}");
            PushFailure::Failed(match failed {
                GitError::NotFastForward { .. } => PushFailureReason::Rejected,
                GitError::AuthenticationFailed { .. } => PushFailureReason::Unauthorized,
                GitError::Network { .. } => PushFailureReason::Network,
                _ => PushFailureReason::Other,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::domain::branch_name::BranchName;
    use crate::git::domain::paths::WorktreePath;
    use crate::git::domain::remote_url::RemoteUrl;
    use crate::git::infra::in_memory::{InMemoryGitRepositories, Operation};

    const API: &str = "/work/api";
    const LOGIN_CHECKOUT: &str = "/work/.worktrees/login/api";
    const MAIN: &str = "main";
    const LOGIN_BRANCH: &str = "feature/login";
    const ORIGIN_URL: &str = "git@github.com:acme/api.git";
    const PUSH: &str = "push";
    const GIT_OUTPUT: &str = "fatal: Authentication failed";

    fn branch(raw: &str) -> BranchName {
        BranchName::parse(raw).unwrap()
    }

    /// api with a remote, and the login branch checked out at its worktree.
    fn git_with_login_checkout() -> Arc<InMemoryGitRepositories> {
        let git = Arc::new(InMemoryGitRepositories::new());
        let api = RepositoryPath::new(API);
        git.add_repository(&api, &branch(MAIN));
        git.set_remote(&api, RemoteUrl::new(ORIGIN_URL));
        let repository = git.open(&api).unwrap();
        repository
            .create_branch(&branch(LOGIN_BRANCH), &branch(MAIN))
            .unwrap();
        repository
            .add_worktree(&WorktreePath::new(LOGIN_CHECKOUT), &branch(LOGIN_BRANCH))
            .unwrap();
        git
    }

    fn pushes(git: &Arc<InMemoryGitRepositories>) -> GitWorktreePushes {
        GitWorktreePushes::new(git.clone())
    }

    #[tokio::test]
    async fn pushes_a_published_branch() {
        let git = git_with_login_checkout();
        git.publish(&RepositoryPath::new(API), &branch(LOGIN_BRANCH));
        git.commit(&RepositoryPath::new(API), &branch(LOGIN_BRANCH));

        assert_eq!(pushes(&git).push(LOGIN_CHECKOUT).await, Ok(()));
    }

    #[tokio::test]
    async fn a_branch_without_upstream_is_published() {
        let git = git_with_login_checkout();

        assert_eq!(pushes(&git).push(LOGIN_CHECKOUT).await, Ok(()));
    }

    #[tokio::test]
    async fn a_repository_without_remote_is_skipped() {
        let git = Arc::new(InMemoryGitRepositories::new());
        git.add_repository(&RepositoryPath::new(API), &branch(MAIN));

        assert_eq!(pushes(&git).push(API).await, Err(PushFailure::Skipped));
    }

    #[tokio::test]
    async fn a_refused_sign_in_fails_as_unauthorized() {
        let git = git_with_login_checkout();
        git.fail_on(
            Operation::Push,
            GitError::AuthenticationFailed {
                operation: PUSH.to_string(),
                output: GIT_OUTPUT.to_string(),
            },
        );

        assert_eq!(
            pushes(&git).push(LOGIN_CHECKOUT).await,
            Err(PushFailure::Failed(PushFailureReason::Unauthorized))
        );
    }

    #[test]
    fn git_failures_map_to_a_reason() {
        let output = || GIT_OUTPUT.to_string();
        let operation = || PUSH.to_string();
        for (error, reason) in [
            (
                GitError::NotFastForward {
                    operation: operation(),
                    output: output(),
                },
                PushFailureReason::Rejected,
            ),
            (
                GitError::Network {
                    operation: operation(),
                    output: output(),
                },
                PushFailureReason::Network,
            ),
            (
                GitError::CommandFailed {
                    operation: operation(),
                    output: output(),
                },
                PushFailureReason::Other,
            ),
        ] {
            assert_eq!(push_failure(error), PushFailure::Failed(reason));
        }
    }
}
