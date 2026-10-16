use serde::Serialize;

use super::identifiers::{RemoteProjectId, TaskGroupId};

/// Kinds of task group. `Epic`, `Sprint` and `Milestone` are added only
/// together with the adapters that produce them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskGroupKind {
    TaskList,
}

/// Read model of a remote grouping of tasks (for example a task list).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGroup {
    pub id: TaskGroupId,
    pub project_id: RemoteProjectId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub kind: TaskGroupKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
