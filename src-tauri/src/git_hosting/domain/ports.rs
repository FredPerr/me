use std::time::Instant;

use async_trait::async_trait;

use super::capped::Capped;
use super::error::{GitHostingError, PushFailureReason};
use super::host_kind::HostKind;
use super::host_secret::HostSecret;
use super::hosted_account::HostedAccount;
use super::hosted_pull_request::HostedPullRequest;
use super::hosted_repository::HostedRepository;
use super::new_pull_request::{CreatedPullRequest, NewPullRequest};
use super::project_pull_requests::{ProjectRepositoryRemote, RepositoryBranchCheckout};
use super::sign_in::{BrowserSignInStep, SignInPoll};

/// The secret store could not be used. Platform details are dropped so they
/// never reach the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostTokensUnavailable;

/// Each Git host's secret, at most one per host.
pub trait HostTokens: Send + Sync {
    fn read(&self, host: HostKind) -> Result<Option<HostSecret>, HostTokensUnavailable>;
    fn save(&self, host: HostKind, secret: &HostSecret) -> Result<(), HostTokensUnavailable>;
    /// Succeeds when no secret is stored.
    fn delete(&self, host: HostKind) -> Result<(), HostTokensUnavailable>;
}

/// A Git host's API, signed in with one account. One adapter per Git host.
#[async_trait]
pub trait GitHost: Send + Sync {
    async fn current_account(&self) -> Result<HostedAccount, GitHostingError>;
    /// Most recently updated first, at most [`PAGE_LIMIT`](super::capped::PAGE_LIMIT) pages.
    async fn list_open_pull_requests(
        &self,
        repository: &HostedRepository,
    ) -> Result<Capped<HostedPullRequest>, GitHostingError>;

    /// The most recently updated pull request from `branch`, whatever its
    /// state, or `None` when the branch has none.
    async fn find_branch_pull_request(
        &self,
        repository: &HostedRepository,
        branch: &str,
    ) -> Result<Option<HostedPullRequest>, GitHostingError>;

    /// Opens `pull_request`, whose head branch is already pushed.
    async fn create_pull_request(
        &self,
        repository: &HostedRepository,
        pull_request: &NewPullRequest,
    ) -> Result<CreatedPullRequest, GitHostingError>;
}

/// The repositories of a Project, their remotes and the branches its contexts
/// use, read from the Workspace and git contexts by the composition root.
#[async_trait]
pub trait ProjectRemotes: Send + Sync {
    /// `NotFound` when no Project has this tag.
    async fn repositories(
        &self,
        project_tag: &str,
    ) -> Result<Vec<ProjectRepositoryRemote>, GitHostingError>;

    /// The remote of one repository of the Project and the checkout of the
    /// context using `branch` in it. `NotFound` when no Project has this tag,
    /// the Project has no such repository, or no context uses the branch there.
    async fn branch_checkout(
        &self,
        project_tag: &str,
        repository_id: &str,
        branch: &str,
    ) -> Result<RepositoryBranchCheckout, GitHostingError>;
}

/// Why a checkout's branch was not pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushFailure {
    /// There is no remote branch to push to.
    Skipped,
    Failed(PushFailureReason),
}

/// Pushes the branch checked out in a worktree to its remote branch, through
/// `git` and the signed-in Git host's credentials.
#[async_trait]
pub trait WorktreePushes: Send + Sync {
    async fn push(&self, worktree_path: &str) -> Result<(), PushFailure>;
}

/// Signing in by entering a code in the browser.
#[async_trait]
pub trait BrowserSignIn: Send + Sync {
    async fn start(&self) -> Result<BrowserSignInStep, GitHostingError>;
    async fn poll(&self, device_code: &str) -> Result<SignInPoll, GitHostingError>;
}

/// The account store could not be used. The reason is logged by the adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostedAccountsUnavailable;

/// The signed-in account summaries, at most one per Git host.
pub trait HostedAccounts: Send + Sync {
    /// Ordered by Git host identifier.
    fn list(&self) -> Result<Vec<HostedAccount>, HostedAccountsUnavailable>;
    // Contract-tested, but no use case reads a single account yet.
    #[allow(dead_code)]
    fn find(&self, host: HostKind) -> Result<Option<HostedAccount>, HostedAccountsUnavailable>;
    /// Replaces the account stored for the same Git host.
    fn save(&self, account: &HostedAccount) -> Result<(), HostedAccountsUnavailable>;
    /// Succeeds when no account is stored.
    fn remove(&self, host: HostKind) -> Result<(), HostedAccountsUnavailable>;
}

/// Tells open views that a Git host was signed in to or out of.
pub trait GitHostingEvents: Send + Sync {
    fn changed(&self, host: HostKind);
}

/// The current time, controllable in tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}
