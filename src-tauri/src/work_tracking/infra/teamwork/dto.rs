//! Teamwork v3 response shapes. Unknown fields are ignored so new Teamwork
//! fields never break parsing.
use std::collections::HashMap;

use serde::{Deserialize, Deserializer};

/// `#[serde(default)]` only covers a missing key; Teamwork may also send
/// `null` for empty collections.
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// The logged-in person. The presence of `person` validates a Teamwork 200
/// body; its `id` identifies the current user for self-assignment.
#[derive(Deserialize)]
pub struct MeResponse {
    pub person: MePersonDto,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MePersonDto {
    pub id: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsResponse {
    #[serde(default, deserialize_with = "null_as_default")]
    pub projects: Vec<ProjectDto>,
    #[serde(default)]
    pub meta: Option<MetaDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasklistsResponse {
    #[serde(default, deserialize_with = "null_as_default")]
    pub tasklists: Vec<TasklistDto>,
    #[serde(default)]
    pub meta: Option<MetaDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasksResponse {
    #[serde(default, deserialize_with = "null_as_default")]
    pub tasks: Vec<TaskDto>,
    #[serde(default)]
    pub included: Option<IncludedDto>,
    #[serde(default)]
    pub meta: Option<MetaDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasklistDto {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub display_order: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parent_task_id: Option<u64>,
    #[serde(default)]
    pub tasklist_id: Option<u64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub progress: Option<u32>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub assignee_user_ids: Vec<u64>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub tag_ids: Vec<u64>,
}

/// Sideloaded users and tags, keyed by stringified id.
#[derive(Default, Deserialize)]
pub struct IncludedDto {
    #[serde(default, deserialize_with = "null_as_default")]
    pub users: HashMap<String, UserDto>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub tags: HashMap<String, TagDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaDto {
    #[serde(default)]
    pub page: Option<PageMetaDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageMetaDto {
    #[serde(default)]
    pub has_more: Option<bool>,
}
