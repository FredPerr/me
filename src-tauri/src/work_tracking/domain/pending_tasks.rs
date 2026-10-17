use std::collections::HashSet;

use async_trait::async_trait;

use super::error::WorkTrackingError;
use super::identifiers::{ConnectionId, RemoteProjectId, TaskId};
use super::task::Task;

/// A remote project attached to a local Project, as the pending-tasks use case
/// needs it: which connection it lives on and which remote project to page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTaskLink {
    pub connection_id: ConnectionId,
    pub remote_project_id: RemoteProjectId,
}

/// The remote project links of a local Project and the tasks its contexts were
/// already created from, read from `workspace` by the composition root so this
/// context never names `workspace`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectLinks {
    /// In the order the Project stores its links.
    pub links: Vec<ProjectTaskLink>,
    /// A context already came from each `(connection, task)` here.
    pub created_from: HashSet<CreatedFromTask>,
}

/// The remote task a context was already created from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CreatedFromTask {
    pub connection_id: ConnectionId,
    pub task_id: TaskId,
}

/// The remote project links of a local Project, read from `workspace`.
#[async_trait]
pub trait ProjectTaskLinks: Send + Sync {
    /// `NotFound` when no Project has this tag.
    async fn links_for(&self, project_tag: &str) -> Result<ProjectLinks, WorkTrackingError>;
}

/// A task still waiting for a context, tagged with where it came from so the
/// frontend can render it and create a context from it.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingTask {
    pub connection_id: ConnectionId,
    pub remote_project_id: RemoteProjectId,
    pub task: Task,
}

/// The pending tasks of a local Project, plus the links that could not be
/// queried because their connection is missing, uncredentialed or unreachable.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PendingTasks {
    /// In link order, each link's open tasks in the order the provider returns them.
    pub tasks: Vec<PendingTask>,
    /// The connections whose links were skipped; reported so the user can reconnect.
    pub unavailable_connections: Vec<ConnectionId>,
}
