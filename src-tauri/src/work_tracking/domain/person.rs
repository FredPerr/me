use serde::Serialize;

use super::identifiers::PersonId;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    id: PersonId,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
}

impl Person {
    /// Keeps `avatar_url` only when it is an `https://` URL.
    pub fn new(id: PersonId, display_name: String, avatar_url: Option<String>) -> Self {
        Self {
            id,
            display_name,
            avatar_url: avatar_url.filter(|url| url.starts_with("https://")),
        }
    }
}
