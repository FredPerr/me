use serde::{Deserialize, Serialize};

use super::identifiers::RepositoryId;

/// One git repository of a Project, at a path relative to the project folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: RepositoryId,
    pub name: String,
    pub relative_path: String,
    pub post_checkout_command: Option<String>,
}

impl Repository {
    pub fn new(
        id: RepositoryId,
        name: impl Into<String>,
        relative_path: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            relative_path: relative_path.into(),
            post_checkout_command: None,
        }
    }
}
