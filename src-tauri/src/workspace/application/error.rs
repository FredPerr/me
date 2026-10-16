use std::fmt;

use crate::git::domain::branch_name::BranchName;
use crate::git::domain::error::GitError;
use crate::workspace::domain::context_name::ContextName;
use crate::workspace::domain::error::WorkspaceError;
use crate::workspace::domain::ports::{CheckoutFilesError, StorageError, UpdateError};
use crate::workspace::domain::project_tag::ProjectTag;

/// Why a workspace use case failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceServiceError {
    ProjectNotFound {
        tag: ProjectTag,
    },
    /// The Project refused the change.
    Rejected(WorkspaceError),
    Storage(StorageError),
    /// A new branch was asked for under a name the repository already has.
    NewBranchAlreadyExists {
        repository: String,
        branch: BranchName,
    },
    BranchCheckedOut {
        repository: String,
        branch: BranchName,
        checked_out_in: CheckoutHolder,
    },
    Git {
        repository: String,
        error: GitError,
    },
    Files(CheckoutFilesError),
}

/// Where a branch is already checked out: a context of the project when the
/// checkout is one of its own, otherwise the folder git reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckoutHolder {
    Context(ContextName),
    Folder(String),
}

impl WorkspaceServiceError {
    pub fn git(repository: &str, error: GitError) -> Self {
        Self::Git {
            repository: repository.to_string(),
            error,
        }
    }
}

impl From<WorkspaceError> for WorkspaceServiceError {
    fn from(error: WorkspaceError) -> Self {
        Self::Rejected(error)
    }
}

impl From<StorageError> for WorkspaceServiceError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<CheckoutFilesError> for WorkspaceServiceError {
    fn from(error: CheckoutFilesError) -> Self {
        Self::Files(error)
    }
}

impl From<UpdateError> for WorkspaceServiceError {
    fn from(error: UpdateError) -> Self {
        match error {
            UpdateError::NotFound { tag } => Self::ProjectNotFound { tag },
            UpdateError::Rejected(error) => Self::Rejected(error),
            UpdateError::Storage(error) => Self::Storage(error),
        }
    }
}

impl fmt::Display for WorkspaceServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectNotFound { tag } => write!(formatter, "Project '{tag}' not found"),
            Self::Rejected(error) => error.fmt(formatter),
            Self::Storage(error) => error.fmt(formatter),
            Self::NewBranchAlreadyExists { repository, branch } => write!(
                formatter,
                "Branch '{branch}' already exists in repository '{repository}': \
                 choose another name or check out the existing branch"
            ),
            Self::BranchCheckedOut {
                repository,
                branch,
                checked_out_in,
            } => match checked_out_in {
                CheckoutHolder::Context(context) => write!(
                    formatter,
                    "Branch '{branch}' of repository '{repository}' is already checked out \
                     in context '{context}'"
                ),
                CheckoutHolder::Folder(folder) => write!(
                    formatter,
                    "Branch '{branch}' of repository '{repository}' is already checked out \
                     in '{folder}'"
                ),
            },
            Self::Git { repository, error } => {
                write!(formatter, "{error} (in repository '{repository}')")
            }
            Self::Files(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WorkspaceServiceError {}
