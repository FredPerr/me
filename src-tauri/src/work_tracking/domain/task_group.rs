use serde::Serialize;

use super::identifiers::{RemoteProjectId, TaskGroupId};

/// Kinds of task group. `Epic`, `Sprint` and `Milestone` are added only
/// together with the adapters that produce them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TaskGroupKind {
    TaskList,
}

/// Read model of a remote grouping of tasks (for example a task list). Options
/// are always emitted (null when absent) so the wire shape stays stable.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaskGroup {
    pub id: TaskGroupId,
    pub project_id: RemoteProjectId,
    pub name: String,
    pub description: Option<String>,
    pub kind: TaskGroupKind,
    pub position: Option<i32>,
    pub url: Option<String>,
}
