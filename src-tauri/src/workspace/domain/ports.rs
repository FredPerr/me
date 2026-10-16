use std::fmt;

use super::error::WorkspaceError;
use super::project::Project;
use super::project_tag::ProjectTag;

/// A change applied to a stored Project by `ProjectRepository::update`.
pub type ProjectChange<'a> = Box<dyn FnOnce(Project) -> Result<Project, WorkspaceError> + 'a>;

/// Where Projects are kept, one record per Project keyed by its tag.
/// Synchronous because redb is synchronous.
pub trait ProjectRepository: Send + Sync {
    /// Every readable Project, ordered by tag, with the records that couldn't be read.
    fn list(&self) -> Result<ProjectListing, StorageError>;

    fn find(&self, tag: &ProjectTag) -> Result<Option<Project>, StorageError>;

    /// Upsert by tag.
    fn save(&self, project: &Project) -> Result<(), StorageError>;

    /// Idempotent.
    fn delete(&self, tag: &ProjectTag) -> Result<(), StorageError>;

    /// Re-reads the Project, applies `change` and saves the result, all in one
    /// write transaction, so concurrent updates never overwrite each other.
    /// Nothing is written when `change` refuses. Returns the saved Project.
    fn update(&self, tag: &ProjectTag, change: ProjectChange<'_>) -> Result<Project, UpdateError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectListing {
    pub projects: Vec<Project>,
    pub unreadable: Vec<UnreadableProject>,
}

/// A stored record that couldn't be decoded into a Project. The tag is the raw
/// record key, which may not be a valid `ProjectTag`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableProject {
    pub tag: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// The database couldn't be opened, read or written.
    Unavailable { reason: String },
    /// The requested Project's record couldn't be decoded.
    UnreadableRecord(UnreadableProject),
    /// The Project couldn't be encoded for saving.
    Unencodable { tag: ProjectTag, reason: String },
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { reason } => {
                write!(formatter, "The project storage is unavailable: {reason}")
            }
            Self::UnreadableRecord(record) => write!(
                formatter,
                "Project '{}' couldn't be loaded: {}",
                record.tag, record.reason
            ),
            Self::Unencodable { tag, reason } => {
                write!(formatter, "Project '{tag}' couldn't be saved: {reason}")
            }
        }
    }
}

impl std::error::Error for StorageError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    NotFound {
        tag: ProjectTag,
    },
    /// The change refused to apply.
    Rejected(WorkspaceError),
    Storage(StorageError),
}

impl From<StorageError> for UpdateError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { tag } => write!(formatter, "Project '{tag}' not found"),
            Self::Rejected(error) => error.fmt(formatter),
            Self::Storage(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for UpdateError {}
