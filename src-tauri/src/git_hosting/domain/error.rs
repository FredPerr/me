/// Input that failed validation. Mirrors the wire message of `invalidInput` errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    /// The Git host does not sign in this way, e.g. a browser sign-in for Bitbucket.
    Host,
    Username,
    Token,
    DeviceCode,
    /// A pull request title.
    Title,
    /// A pull request base branch: missing, or the same as its head branch.
    BaseBranch,
}

/// Why pushing a branch before opening its pull request failed. git's own
/// output is only logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushFailureReason {
    /// The remote has commits the branch does not.
    Rejected,
    Unauthorized,
    Network,
    Other,
}

/// Closed set of failures of the Git Hosting context. No variant carries a
/// free-form string, so response bodies and secrets can never leak through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHostingError {
    NotConnected,
    InvalidInput {
        field: InputField,
    },
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited {
        retry_after_seconds: Option<u32>,
    },
    Network,
    Timeout,
    ProviderError {
        status: Option<u16>,
    },
    InvalidResponse,
    Storage,
    /// The repository has no `origin` remote.
    NoRemote,
    /// The repository's remote is not on a supported Git host.
    UnsupportedHost,
    /// The branch was not pushed: it has no remote branch to push to.
    PushSkipped,
    PushFailed {
        reason: PushFailureReason,
    },
}

impl GitHostingError {
    pub fn invalid_input(field: InputField) -> Self {
        Self::InvalidInput { field }
    }
}
