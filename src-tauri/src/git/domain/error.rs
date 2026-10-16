use std::fmt;

/// Why a git operation failed. Variants that come from a failed git command
/// keep git's own output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    NotARepository { path: String },
    InvalidBranchName { name: String, reason: &'static str },
    BranchNotFound { name: String },
    BranchAlreadyExists { name: String },
    BranchCheckedOut { name: String, worktree: String },
    NoRemote,
    NoUpstream,
    DetachedHead,
    NotFastForward { operation: String, output: String },
    AuthenticationFailed { operation: String, output: String },
    Network { operation: String, output: String },
    Io { action: String, detail: String },
    CommandFailed { operation: String, output: String },
}

impl GitError {
    /// git's own output, when the error came from a failed git command.
    pub fn git_output(&self) -> Option<&str> {
        match self {
            Self::NotFastForward { output, .. }
            | Self::AuthenticationFailed { output, .. }
            | Self::Network { output, .. }
            | Self::CommandFailed { output, .. } => Some(output),
            _ => None,
        }
    }

    pub(crate) fn io(action: impl Into<String>, error: impl fmt::Display) -> Self {
        Self::Io {
            action: action.into(),
            detail: error.to_string(),
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotARepository { path } => write!(formatter, "No git repository at '{path}'"),
            Self::InvalidBranchName { name, reason } => {
                write!(formatter, "Invalid branch name '{name}': it {reason}")
            }
            Self::BranchNotFound { name } => write!(formatter, "Branch '{name}' not found"),
            Self::BranchAlreadyExists { name } => {
                write!(formatter, "Branch '{name}' already exists")
            }
            Self::BranchCheckedOut { name, worktree } => {
                write!(formatter, "Branch '{name}' is checked out in '{worktree}'")
            }
            Self::NoRemote => formatter.write_str("No remote configured"),
            Self::NoUpstream => formatter.write_str("No upstream branch (not pushed yet)"),
            Self::DetachedHead => formatter.write_str("HEAD is detached"),
            Self::Io { action, detail } => write!(formatter, "Failed to {action}: {detail}"),
            Self::NotFastForward { operation, output }
            | Self::AuthenticationFailed { operation, output }
            | Self::Network { operation, output }
            | Self::CommandFailed { operation, output } => {
                write!(formatter, "git {operation} failed: {output}")
            }
        }
    }
}

impl std::error::Error for GitError {}

/// Commands still report errors as text; the typed wire format comes later.
impl From<GitError> for String {
    fn from(error: GitError) -> Self {
        error.to_string()
    }
}
