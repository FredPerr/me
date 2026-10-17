use serde::Serialize;

use super::identifiers::{ConnectionId, RemoteProjectId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RemoteProjectStatus {
    Active,
    Archived,
    Other,
}

/// Read model of a remote project; never persisted locally. Options are always
/// emitted (null when absent) so the wire shape stays stable for specta.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteProject {
    pub id: RemoteProjectId,
    pub connection_id: ConnectionId,
    pub name: String,
    pub description: Option<String>,
    pub status: RemoteProjectStatus,
    pub url: String,
}
