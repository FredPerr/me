use serde::Serialize;

use super::identifiers::{WorkItemGroupId, WorkItemId, WorkProjectId};
use super::label::Label;
use super::person::Person;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkItemStatus {
    Todo,
    InProgress,
    Done,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkItemPriority {
    None,
    Low,
    Medium,
    High,
    #[allow(dead_code)] // produced by future Jira adapter (Highest)
    Urgent,
    Unknown,
}

/// Read model of a remote task, issue or subtask.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItem {
    pub id: WorkItemId,
    pub project_id: WorkProjectId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<WorkItemGroupId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<WorkItemId>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: WorkItemStatus,
    pub raw_status: String,
    pub priority: WorkItemPriority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_priority: Option<String>,
    pub assignees: Vec<Person>,
    pub labels: Vec<Label>,
    /// Always `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    pub url: String,
    /// RFC 3339 as received from the provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}
