use std::fmt;

use super::identifiers::{ContextId, RepositoryId};

/// Why a branch prefix was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixRejection {
    Empty,
    Invalid,
}

/// Why a change to a Project was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    EmptyProjectTag,
    EmptyProjectName,
    EmptyProjectPath,
    EmptyIdentifier,
    EmptyContextName,
    /// The name has no letter or digit, so it gives no folder under `.worktrees/`.
    EmptyContextFolder {
        name: String,
    },
    DuplicateContextName {
        name: String,
    },
    DuplicateContextFolder {
        folder: String,
    },
    UnknownContext {
        id: ContextId,
    },
    UnknownBaseContext {
        id: ContextId,
    },
    UnknownRepository {
        id: RepositoryId,
    },
    DuplicateContextBranch {
        repository: RepositoryId,
    },
    DuplicateRepository {
        id: RepositoryId,
    },
    UnknownContextBranch {
        context: ContextId,
        repository: RepositoryId,
    },
    DefaultContextAlreadyExists,
    DefaultContextIsFixed,
    StaticContextStatusLocked {
        id: ContextId,
    },
    LinkedRepositoryIsReadOnly {
        repository: RepositoryId,
    },
    InvalidBranchName {
        name: String,
        reason: &'static str,
    },
    EmptyBranchName {
        source: String,
    },
    InvalidPrefix {
        prefix: String,
        reason: PrefixRejection,
    },
    UnknownPrefix {
        prefix: String,
    },
    PrefixRequired,
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProjectTag => formatter.write_str("The project tag is empty"),
            Self::EmptyProjectName => formatter.write_str("The project name is empty"),
            Self::EmptyProjectPath => formatter.write_str("The project folder is empty"),
            Self::EmptyIdentifier => formatter.write_str("The identifier is empty"),
            Self::EmptyContextName => formatter.write_str("The context name is empty"),
            Self::EmptyContextFolder { name } => write!(
                formatter,
                "The context name '{name}' needs at least one letter or digit"
            ),
            Self::DuplicateContextName { name } => {
                write!(formatter, "A context named '{name}' already exists")
            }
            Self::DuplicateContextFolder { folder } => {
                write!(formatter, "A context already uses the folder '{folder}'")
            }
            Self::UnknownContext { id } => write!(formatter, "Context '{id}' not found"),
            Self::UnknownBaseContext { id } => {
                write!(formatter, "Base context '{id}' not found")
            }
            Self::UnknownRepository { id } => write!(formatter, "Repository '{id}' not found"),
            Self::DuplicateContextBranch { repository } => write!(
                formatter,
                "The context has more than one branch for repository '{repository}'"
            ),
            Self::DuplicateRepository { id } => write!(
                formatter,
                "The project has more than one repository with the id '{id}'"
            ),
            Self::UnknownContextBranch {
                context,
                repository,
            } => write!(
                formatter,
                "Context '{context}' has no branch for repository '{repository}'"
            ),
            Self::DefaultContextAlreadyExists => {
                formatter.write_str("The project already has a default context")
            }
            Self::DefaultContextIsFixed => {
                formatter.write_str("The default context can't be deleted or changed")
            }
            Self::StaticContextStatusLocked { id } => {
                write!(formatter, "Context '{id}' is static and keeps its status")
            }
            Self::LinkedRepositoryIsReadOnly { repository } => write!(
                formatter,
                "Repository '{repository}' is a linked repository, which can only be deleted"
            ),
            Self::InvalidBranchName { name, reason } => {
                write!(formatter, "Invalid branch name '{name}': it {reason}")
            }
            Self::EmptyBranchName { source } => write!(
                formatter,
                "'{source}' gives an empty branch name: it needs at least one letter or digit"
            ),
            Self::InvalidPrefix { prefix, reason } => match reason {
                PrefixRejection::Empty => formatter.write_str("The branch prefix is empty"),
                PrefixRejection::Invalid => {
                    write!(formatter, "'{prefix}' is not a valid branch prefix")
                }
            },
            Self::UnknownPrefix { prefix } => write!(formatter, "Unknown branch prefix: {prefix}"),
            Self::PrefixRequired => {
                formatter.write_str("A branch prefix is required by this project")
            }
        }
    }
}

impl std::error::Error for WorkspaceError {}
