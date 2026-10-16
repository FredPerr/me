#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestState {
    Open,
    Merged,
    Closed,
}

/// A pull request of a repository on a Git host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedPullRequest {
    pub number: u32,
    pub title: String,
    pub url: String,
    pub author: Option<String>,
    pub head_branch: String,
    pub base_branch: String,
    /// Only ever set while the pull request is open.
    pub draft: bool,
    pub state: PullRequestState,
    pub updated_at: String,
}
