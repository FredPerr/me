use serde::Serialize;

use super::identifiers::{ConnectionId, WorkProjectId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkProjectStatus {
    Active,
    Archived,
    Other,
}

/// Read model of a remote project; never persisted locally.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkProject {
    pub id: WorkProjectId,
    pub connection_id: ConnectionId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: WorkProjectStatus,
    pub url: String,
}
