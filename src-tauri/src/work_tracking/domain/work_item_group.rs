use serde::Serialize;

use super::identifiers::{WorkItemGroupId, WorkProjectId};

/// Kinds of item groups. `Epic`, `Sprint` and `Milestone` are added only
/// together with the adapters that produce them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkItemGroupKind {
    TaskList,
}

/// Read model of a remote grouping of items (for example a task list).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemGroup {
    pub id: WorkItemGroupId,
    pub project_id: WorkProjectId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub kind: WorkItemGroupKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
