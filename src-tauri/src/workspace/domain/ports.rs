use std::fmt;
use std::path::Path;

use super::agent_run::{AgentRun, AgentRunState};
use super::agent_work::AgentTool;
use super::error::WorkspaceError;
use super::identifiers::{AgentRunId, ContextId};
use super::project::Project;
use super::project_tag::ProjectTag;
use super::workbench::Workbench;

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

/// Where the one Workbench is kept, a single record in its own table. An
/// absent or unreadable record reads as `Workbench::default()`, so the bar
/// never breaks; reconcile against the live project list then folds it back
/// into shape. Synchronous because redb is synchronous.
pub trait WorkbenchRepository: Send + Sync {
    /// The stored Workbench, or `Workbench::default()` when there is no record
    /// or the record can't be read.
    fn load(&self) -> Result<Workbench, StorageError>;

    /// Writes the one Workbench record, replacing any previous one.
    fn save(&self, workbench: &Workbench) -> Result<(), StorageError>;
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

/// The steps of a context checkout on disk that aren't git: the project's
/// configured symlinks and the repositories' post-checkout commands. A port so
/// the service's undo and ordering can be tested without touching the disk.
pub trait CheckoutFiles: Send + Sync {
    /// Links `link` to `target`, creating its parent folders. Returns `false`
    /// and leaves things as they are when something is already at `link`.
    fn create_symlink(&self, target: &Path, link: &Path) -> Result<bool, CheckoutFilesError>;
    /// Removes the symlink at `link`; does nothing when there is no symlink there.
    fn remove_symlink(&self, link: &Path) -> Result<(), CheckoutFilesError>;
    /// Removes `folder` when it exists and is empty.
    fn remove_folder_if_empty(&self, folder: &Path);
    /// Runs a repository's post-checkout command (e.g. installing
    /// dependencies) in `checkout`. A failing command doesn't fail the
    /// context: the checkout is still usable.
    fn run_post_checkout(&self, command: &str, checkout: &Path);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutFilesError {
    pub path: String,
    pub reason: String,
}

impl fmt::Display for CheckoutFilesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "'{}' couldn't be changed: {}",
            self.path, self.reason
        )
    }
}

impl std::error::Error for CheckoutFilesError {}

/// Tells every open view that a Project changed.
pub trait ProjectEvents: Send + Sync {
    fn project_changed(&self, tag: &ProjectTag);
}

/// The wall-clock time a run started, as an ISO 8601 string. A port so the
/// run-driving use case records `AgentRun::started_at` from a clock the tests
/// can fix, matching the other timestamp fields stored as strings.
pub trait Clock: Send + Sync {
    /// The current instant formatted as ISO 8601, e.g. `2024-01-15T10:00:00Z`.
    fn now_iso8601(&self) -> String;
}

/// Seeds the Kiro permission "floor" in a run's working directory before a
/// trusted (`--trust-all-tools`) run: a `deny` policy Kiro cannot override, so
/// dangerous commands and secret-file access stay blocked. Ported from the
/// frontend's `ensurePermissionFloor`. A port so the use case stays off the
/// disk in tests; the file-system adapter writes the policy once, never
/// overwriting a user's own edits.
pub trait PermissionFloor: Send + Sync {
    /// Ensures the policy exists in `working_directory`. Failures never block a
    /// run, so this returns nothing; the adapter logs what it could not write.
    fn seed(&self, working_directory: &str);
}

/// The AI tool a brand-new context's first run uses, read from the app's
/// Default AI tool setting. A context with no prior [`AgentWork`] has no
/// recorded tool yet, so the run-driving use case reads this to decide which
/// tool to start (and records it on the context so follow-up runs keep it).
///
/// A port so `workspace` never imports `settings`: a composition adapter reads
/// the setting and maps it to the parallel domain [`AgentTool`]. When the
/// setting cannot be read the adapter falls back to the default tool, so a read
/// failure never blocks a run.
pub trait DefaultAgentTool: Send + Sync {
    /// The default tool to start a fresh context's first run with, as a tool
    /// with no conversation recorded yet.
    fn default_tool(&self) -> AgentTool;
}

/// Where a context's agent-run transcripts are kept, one record per context
/// holding its recent runs oldest first. The two caps (200 KB per transcript,
/// the last ten runs per context) are enforced on write, so a stored context
/// never holds more than ten runs and no transcript exceeds its size cap.
/// Synchronous because redb is synchronous. The run-driving use case in ticket
/// #80 is the only caller; it starts a run, streams lines into it and ends it.
pub trait AgentRunRepository: Send + Sync {
    /// Records a freshly started run as the newest run of its context, then
    /// drops the context's oldest runs beyond the last-ten cap.
    fn start_run(&self, run: &AgentRun) -> Result<(), StorageError>;

    /// Appends output lines to a run, re-applying the transcript size cap.
    /// Does nothing when the run is unknown (e.g. already dropped by the cap).
    fn append_lines(
        &self,
        context_id: &ContextId,
        run_id: &AgentRunId,
        lines: &[String],
    ) -> Result<(), StorageError>;

    /// Records how a run ended and its exit code. Does nothing for an unknown run.
    fn finish_run(
        &self,
        context_id: &ContextId,
        run_id: &AgentRunId,
        state: AgentRunState,
        exit_code: Option<i32>,
    ) -> Result<(), StorageError>;

    /// A context's runs, oldest first. Empty for a context with no runs or an
    /// unreadable record.
    fn list_runs(&self, context_id: &ContextId) -> Result<Vec<AgentRun>, StorageError>;

    /// Removes every run of a context, hooked into deleting the context.
    /// Idempotent.
    fn delete_runs(&self, context_id: &ContextId) -> Result<(), StorageError>;

    /// Every run left in the `Running` state across all contexts, so the next
    /// start after a crash can reconcile them (ticket #82). Oldest first within
    /// each context; contexts in storage order.
    fn running_runs(&self) -> Result<Vec<AgentRun>, StorageError>;
}
