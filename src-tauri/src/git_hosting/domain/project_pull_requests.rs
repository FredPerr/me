use crate::git::domain::remote_url::RemoteUrl;

use super::capped::Capped;
use super::error::GitHostingError;
use super::host_kind::HostKind;
use super::hosted_pull_request::HostedPullRequest;

/// A repository of a Project, its `origin` remote if it has one, and the
/// branch each context of the Project uses in it, in context order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRepositoryRemote {
    pub repository_id: String,
    pub repository_name: String,
    pub remote: Option<RemoteUrl>,
    pub context_branches: Vec<String>,
}

/// The remote of a repository of a Project, and where a context of the
/// Project checks one of its branches out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryBranchCheckout {
    pub remote: Option<RemoteUrl>,
    pub worktree_path: String,
}

/// What became of asking a repository's Git host for pull requests. `Found`
/// is what an answer holds: the open pull requests of the repository by
/// default, or the pull request of one branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullRequestsStatus<Found = Capped<HostedPullRequest>> {
    NoRemote,
    /// The remote is not on a supported Git host, or could not be read.
    UnsupportedHost,
    NotConnected,
    Failed(GitHostingError),
    Ok(Found),
}

/// The most recently updated pull request of a branch, if it has one.
pub type BranchPullRequestStatus = PullRequestsStatus<Option<HostedPullRequest>>;

/// The open pull requests of one repository of a Project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPullRequests {
    pub repository_id: String,
    pub repository_name: String,
    /// The Git host of the remote, when it is a supported one.
    pub host: Option<HostKind>,
    pub status: PullRequestsStatus,
}

/// The pull request of the branch a context uses in one repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchPullRequest {
    pub repository_id: String,
    pub branch: String,
    /// The Git host of the remote, when it is a supported one.
    pub host: Option<HostKind>,
    pub status: BranchPullRequestStatus,
}
