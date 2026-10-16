use crate::git::domain::remote_url::RemoteUrl;

use super::capped::Capped;
use super::error::GitHostingError;
use super::host_kind::HostKind;
use super::hosted_pull_request::HostedPullRequest;

/// A repository of a Project and its `origin` remote, if it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRepositoryRemote {
    pub repository_id: String,
    pub repository_name: String,
    pub remote: Option<RemoteUrl>,
}

/// What became of loading one repository's open pull requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullRequestsStatus {
    NoRemote,
    /// The remote is not on a supported Git host, or could not be read.
    UnsupportedHost,
    NotConnected,
    Failed(GitHostingError),
    Ok(Capped<HostedPullRequest>),
}

/// The open pull requests of one repository of a Project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPullRequests {
    pub repository_id: String,
    pub repository_name: String,
    /// The Git host of the remote, when it is a supported one.
    pub host: Option<HostKind>,
    pub status: PullRequestsStatus,
}
