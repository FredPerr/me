use serde::Serialize;

use super::identifiers::{RemoteProjectId, TaskGroupId, TaskId};
use super::label::Label;
use super::person::Person;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TaskPriority {
    None,
    Low,
    Medium,
    High,
    #[allow(dead_code)] // produced by future Jira adapter (Highest)
    Urgent,
    Unknown,
}

/// Read model of a remote task, issue or subtask. Options are always emitted
/// (null when absent) so the wire shape stays stable for specta.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: TaskId,
    pub project_id: RemoteProjectId,
    pub group_id: Option<TaskGroupId>,
    pub parent_id: Option<TaskId>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub raw_status: String,
    pub priority: TaskPriority,
    pub raw_priority: Option<String>,
    pub assignees: Vec<Person>,
    pub labels: Vec<Label>,
    /// Always `YYYY-MM-DD`.
    pub due_date: Option<String>,
    pub url: String,
    /// RFC 3339 as received from the provider.
    pub updated_at: Option<String>,
}
