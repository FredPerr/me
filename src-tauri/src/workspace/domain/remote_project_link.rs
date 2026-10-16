use serde::{Deserialize, Serialize};

/// A remote project attached to a Project. Carried as saved by the frontend;
/// its rules stay with work tracking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteProjectLink {
    pub provider_kind: String,
    pub connection_id: String,
    pub remote_project_id: String,
    pub remote_project_name: String,
    pub remote_project_url: Option<String>,
    pub linked_at: String,
}
