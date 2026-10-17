//! Typed Tauri commands of the Work Tracking context and their wire types.
//! Secrets never leave Rust: the API key only passes through `save_connection`
//! and is never returned.
use serde::{Deserialize, Serialize};
use tauri::State;

use super::application::service::{ConnectionView, SaveConnectionCommand, WorkTrackingService};
use super::domain::error::{InputField, InvalidInputReason, WorkTrackingError};
use super::domain::identifiers::{ConnectionId, RemoteProjectId};
use super::domain::page::Page;
use super::domain::pending_tasks::{PendingTask, PendingTasks};
use super::domain::remote_project::RemoteProject;
use super::domain::task::Task;
use super::domain::task_group::TaskGroup;

/// The kind of a failed Work Tracking command, on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkTrackingErrorKind {
    NotConfigured,
    InvalidInput,
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited,
    Timeout,
    Network,
    ProviderError,
    InvalidResponse,
    StorageError,
}

/// The input that failed validation, on the wire. Mirrors [`InputField`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkTrackingField {
    ProviderKind,
    BaseUrl,
    ApiKey,
    DisplayName,
    ConnectionId,
    ProjectId,
    GroupId,
    ItemId,
    Cursor,
    PageSize,
}

impl From<InputField> for WorkTrackingField {
    fn from(field: InputField) -> Self {
        match field {
            InputField::ProviderKind => Self::ProviderKind,
            InputField::BaseUrl => Self::BaseUrl,
            InputField::ApiKey => Self::ApiKey,
            InputField::DisplayName => Self::DisplayName,
            InputField::ConnectionId => Self::ConnectionId,
            InputField::ProjectId => Self::ProjectId,
            InputField::GroupId => Self::GroupId,
            InputField::ItemId => Self::ItemId,
            InputField::Cursor => Self::Cursor,
            InputField::PageSize => Self::PageSize,
        }
    }
}

/// Why an input failed validation, on the wire. Mirrors [`InvalidInputReason`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkTrackingInvalidReason {
    Required,
    TooLong,
    InvalidFormat,
    HttpsRequired,
    UnsupportedComponent,
    OutOfRange,
    Unsupported,
}

impl From<InvalidInputReason> for WorkTrackingInvalidReason {
    fn from(reason: InvalidInputReason) -> Self {
        match reason {
            InvalidInputReason::Required => Self::Required,
            InvalidInputReason::TooLong => Self::TooLong,
            InvalidInputReason::InvalidFormat => Self::InvalidFormat,
            InvalidInputReason::HttpsRequired => Self::HttpsRequired,
            InvalidInputReason::UnsupportedComponent => Self::UnsupportedComponent,
            InvalidInputReason::OutOfRange => Self::OutOfRange,
            InvalidInputReason::Unsupported => Self::Unsupported,
        }
    }
}

/// A failed Work Tracking command on the wire. `message` is a fixed English
/// sentence per kind and never holds provider or credential data. Every field
/// is emitted (null when absent) so the frontend reads one stable shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkTrackingCommandError {
    pub kind: WorkTrackingErrorKind,
    pub message: String,
    pub field: Option<WorkTrackingField>,
    pub reason: Option<WorkTrackingInvalidReason>,
    pub retry_after_seconds: Option<u32>,
    pub status: Option<u16>,
}

impl WorkTrackingCommandError {
    fn new(kind: WorkTrackingErrorKind, message: &str) -> Self {
        Self {
            kind,
            message: message.to_string(),
            field: None,
            reason: None,
            retry_after_seconds: None,
            status: None,
        }
    }
}

impl From<WorkTrackingError> for WorkTrackingCommandError {
    fn from(error: WorkTrackingError) -> Self {
        use WorkTrackingErrorKind as Kind;
        match error {
            WorkTrackingError::NotConfigured => {
                Self::new(Kind::NotConfigured, "The connection is not configured.")
            }
            WorkTrackingError::InvalidInput { field, reason } => Self {
                field: Some(field.into()),
                reason: Some(reason.into()),
                ..Self::new(Kind::InvalidInput, "The input is invalid.")
            },
            WorkTrackingError::Unauthorized => {
                Self::new(Kind::Unauthorized, "The provider rejected the credentials.")
            }
            WorkTrackingError::Forbidden => {
                Self::new(Kind::Forbidden, "Access to this resource is forbidden.")
            }
            WorkTrackingError::NotFound => {
                Self::new(Kind::NotFound, "The requested resource was not found.")
            }
            WorkTrackingError::RateLimited {
                retry_after_seconds,
            } => Self {
                retry_after_seconds: retry_after_seconds.map(|seconds| seconds as u32),
                ..Self::new(Kind::RateLimited, "The provider rate limit was reached.")
            },
            WorkTrackingError::Timeout => {
                Self::new(Kind::Timeout, "The request to the provider timed out.")
            }
            WorkTrackingError::Network => {
                Self::new(Kind::Network, "The provider could not be reached.")
            }
            WorkTrackingError::ProviderError { status } => Self {
                status,
                ..Self::new(
                    Kind::ProviderError,
                    "The provider returned an unexpected error.",
                )
            },
            WorkTrackingError::InvalidResponse => Self::new(
                Kind::InvalidResponse,
                "The provider returned an invalid response.",
            ),
            WorkTrackingError::StorageError => {
                Self::new(Kind::StorageError, "Local storage could not be accessed.")
            }
        }
    }
}

// No Debug derive: holds the raw API key. Fields are moved uninspected into
// `SaveConnectionCommand`; the service validates them. The type is `pub` only
// because `#[tauri::command]` requires it; its fields stay private.
#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveConnectionInput {
    kind: String,
    base_url: String,
    #[serde(default)]
    display_name: Option<String>,
    api_key: String,
}

impl From<SaveConnectionInput> for SaveConnectionCommand {
    fn from(input: SaveConnectionInput) -> Self {
        Self {
            kind: input.kind,
            base_url: input.base_url,
            display_name: input.display_name,
            api_key: input.api_key,
        }
    }
}

/// A task still waiting for a context: a dashed card in the board's Idle
/// column. Carries its connection and remote project so the frontend can show
/// it and create a context from it.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PendingTaskView {
    pub connection_id: ConnectionId,
    pub remote_project_id: RemoteProjectId,
    pub task: Task,
}

impl From<PendingTask> for PendingTaskView {
    fn from(pending: PendingTask) -> Self {
        Self {
            connection_id: pending.connection_id,
            remote_project_id: pending.remote_project_id,
            task: pending.task,
        }
    }
}

/// The board's pending task cards for one Project, plus the connections whose
/// links were skipped because they are missing, uncredentialed or unreachable.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PendingTasksView {
    pub tasks: Vec<PendingTaskView>,
    pub unavailable_connections: Vec<ConnectionId>,
}

impl From<PendingTasks> for PendingTasksView {
    fn from(pending: PendingTasks) -> Self {
        Self {
            tasks: pending
                .tasks
                .into_iter()
                .map(PendingTaskView::from)
                .collect(),
            unavailable_connections: pending.unavailable_connections,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn work_tracking_list_connections(
    state: State<'_, WorkTrackingService>,
) -> Result<Vec<ConnectionView>, WorkTrackingCommandError> {
    Ok(state.list_connections()?)
}

#[tauri::command]
#[specta::specta]
pub async fn work_tracking_save_connection(
    state: State<'_, WorkTrackingService>,
    input: SaveConnectionInput,
) -> Result<ConnectionView, WorkTrackingCommandError> {
    Ok(state.save_connection(input.into()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn work_tracking_remove_connection(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
) -> Result<(), WorkTrackingCommandError> {
    Ok(state.remove_connection(&connection_id)?)
}

#[tauri::command]
#[specta::specta]
pub async fn work_tracking_list_projects(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
    cursor: Option<String>,
) -> Result<Page<RemoteProject>, WorkTrackingCommandError> {
    Ok(state
        .list_projects(&connection_id, cursor.as_deref())
        .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn work_tracking_list_groups(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
    project_id: String,
    cursor: Option<String>,
) -> Result<Page<TaskGroup>, WorkTrackingCommandError> {
    Ok(state
        .list_groups(&connection_id, &project_id, cursor.as_deref())
        .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn work_tracking_list_items(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
    project_id: String,
    group_id: String,
    cursor: Option<String>,
) -> Result<Page<Task>, WorkTrackingCommandError> {
    Ok(state
        .list_items(&connection_id, &project_id, &group_id, cursor.as_deref())
        .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn work_tracking_list_project_items(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
    project_id: String,
    cursor: Option<String>,
) -> Result<Page<Task>, WorkTrackingCommandError> {
    Ok(state
        .list_project_items(&connection_id, &project_id, cursor.as_deref())
        .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn work_tracking_assign_me_to_task(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
    item_id: String,
) -> Result<(), WorkTrackingCommandError> {
    Ok(state.assign_me_to_task(&connection_id, &item_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn work_tracking_pending_tasks(
    state: State<'_, WorkTrackingService>,
    project_tag: String,
) -> Result<PendingTasksView, WorkTrackingCommandError> {
    Ok(state.pending_tasks(&project_tag).await?.into())
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    fn wire(error: WorkTrackingError) -> Value {
        serde_json::to_value(WorkTrackingCommandError::from(error)).unwrap()
    }

    #[test]
    fn unauthorized_emits_every_field_with_nulls() {
        assert_eq!(
            wire(WorkTrackingError::Unauthorized),
            json!({
                "kind": "unauthorized",
                "message": "The provider rejected the credentials.",
                "field": null,
                "reason": null,
                "retryAfterSeconds": null,
                "status": null,
            })
        );
    }

    #[test]
    fn invalid_input_carries_field_and_reason() {
        let value = wire(WorkTrackingError::invalid_input(
            InputField::BaseUrl,
            InvalidInputReason::HttpsRequired,
        ));
        assert_eq!(value["kind"], "invalidInput");
        assert_eq!(value["field"], "baseUrl");
        assert_eq!(value["reason"], "httpsRequired");
        assert_eq!(value["status"], Value::Null);
        assert_eq!(value["retryAfterSeconds"], Value::Null);
    }

    #[test]
    fn rate_limited_and_provider_error_details() {
        let limited = wire(WorkTrackingError::RateLimited {
            retry_after_seconds: Some(30),
        });
        assert_eq!(limited["kind"], "rateLimited");
        assert_eq!(limited["retryAfterSeconds"], 30);
        let limited_without_delay = wire(WorkTrackingError::RateLimited {
            retry_after_seconds: None,
        });
        assert_eq!(limited_without_delay["retryAfterSeconds"], Value::Null);
        let provider = wire(WorkTrackingError::ProviderError { status: Some(503) });
        assert_eq!(provider["kind"], "providerError");
        assert_eq!(provider["status"], 503);
    }

    #[test]
    fn every_kind_maps_to_its_wire_name() {
        let cases = [
            (WorkTrackingError::NotConfigured, "notConfigured"),
            (WorkTrackingError::Unauthorized, "unauthorized"),
            (WorkTrackingError::Forbidden, "forbidden"),
            (WorkTrackingError::NotFound, "notFound"),
            (WorkTrackingError::Timeout, "timeout"),
            (WorkTrackingError::Network, "network"),
            (
                WorkTrackingError::ProviderError { status: None },
                "providerError",
            ),
            (WorkTrackingError::InvalidResponse, "invalidResponse"),
            (WorkTrackingError::StorageError, "storageError"),
        ];
        for (error, kind) in cases {
            let value = wire(error);
            assert_eq!(value["kind"], kind);
            assert!(value["message"]
                .as_str()
                .is_some_and(|message| !message.is_empty()));
            // Six keys always present, all null for these variants.
            assert_eq!(value.as_object().unwrap().len(), 6, "{value}");
            assert_eq!(value["field"], Value::Null);
            assert_eq!(value["reason"], Value::Null);
        }
    }

    #[test]
    fn every_field_and_reason_maps_to_a_camel_case_name() {
        let field = |field| {
            serde_json::to_value(WorkTrackingField::from(field))
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(field(InputField::ProviderKind), "providerKind");
        assert_eq!(field(InputField::ApiKey), "apiKey");
        assert_eq!(field(InputField::DisplayName), "displayName");
        assert_eq!(field(InputField::ConnectionId), "connectionId");
        assert_eq!(field(InputField::ProjectId), "projectId");
        assert_eq!(field(InputField::GroupId), "groupId");
        assert_eq!(field(InputField::ItemId), "itemId");
        assert_eq!(field(InputField::Cursor), "cursor");
        assert_eq!(field(InputField::PageSize), "pageSize");

        let reason = |reason| {
            serde_json::to_value(WorkTrackingInvalidReason::from(reason))
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(reason(InvalidInputReason::Required), "required");
        assert_eq!(reason(InvalidInputReason::TooLong), "tooLong");
        assert_eq!(reason(InvalidInputReason::InvalidFormat), "invalidFormat");
        assert_eq!(reason(InvalidInputReason::HttpsRequired), "httpsRequired");
        assert_eq!(
            reason(InvalidInputReason::UnsupportedComponent),
            "unsupportedComponent"
        );
        assert_eq!(reason(InvalidInputReason::OutOfRange), "outOfRange");
        assert_eq!(reason(InvalidInputReason::Unsupported), "unsupported");
    }

    #[test]
    fn save_input_deserializes_camel_case() {
        let input: SaveConnectionInput = serde_json::from_str(
            r#"{"kind":"teamwork","baseUrl":"https://a.teamwork.com","apiKey":"k"}"#,
        )
        .unwrap();
        assert_eq!(input.kind, "teamwork");
        assert_eq!(input.base_url, "https://a.teamwork.com");
        assert_eq!(input.api_key, "k");
        assert_eq!(input.display_name, None);

        let named: SaveConnectionInput = serde_json::from_str(
            r#"{"kind":"teamwork","baseUrl":"https://a.teamwork.com","displayName":"Acme","apiKey":"k"}"#,
        )
        .unwrap();
        assert_eq!(named.display_name.as_deref(), Some("Acme"));
        let command = SaveConnectionCommand::from(named);
        assert_eq!(command.display_name.as_deref(), Some("Acme"));
        assert_eq!(command.api_key, "k");
    }

    #[test]
    fn pending_tasks_view_wire_shape() {
        use crate::work_tracking::domain::identifiers::{RemoteProjectId, TaskId};
        use crate::work_tracking::domain::task::{Task, TaskPriority, TaskStatus};

        let task = Task {
            id: TaskId::from_numeric(1),
            project_id: RemoteProjectId::from_numeric(42),
            group_id: None,
            parent_id: None,
            title: "Add login".into(),
            description: None,
            status: TaskStatus::Todo,
            raw_status: "new".into(),
            priority: TaskPriority::None,
            raw_priority: None,
            assignees: vec![],
            labels: vec![],
            due_date: None,
            url: "https://acme.teamwork.com/app/tasks/1".into(),
            updated_at: None,
        };
        let view = PendingTasksView::from(PendingTasks {
            tasks: vec![PendingTask {
                connection_id: ConnectionId::parse(
                    "teamwork:acme.teamwork.com",
                    InputField::ConnectionId,
                )
                .unwrap(),
                remote_project_id: RemoteProjectId::from_numeric(42),
                task,
            }],
            unavailable_connections: vec![ConnectionId::parse(
                "teamwork:other.teamwork.com",
                InputField::ConnectionId,
            )
            .unwrap()],
        });
        let value = serde_json::to_value(&view).unwrap();
        assert_eq!(
            value["tasks"][0]["connectionId"],
            "teamwork:acme.teamwork.com"
        );
        assert_eq!(value["tasks"][0]["remoteProjectId"], "42");
        assert_eq!(value["tasks"][0]["task"]["id"], "1");
        assert_eq!(value["tasks"][0]["task"]["title"], "Add login");
        assert_eq!(
            value["unavailableConnections"][0],
            "teamwork:other.teamwork.com"
        );
    }

    #[test]
    fn save_input_rejects_unknown_and_missing_fields() {
        assert!(serde_json::from_str::<SaveConnectionInput>(
            r#"{"kind":"teamwork","base_url":"https://a.teamwork.com","baseUrl":"https://a.teamwork.com","apiKey":"k"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<SaveConnectionInput>(
            r#"{"kind":"teamwork","baseUrl":"https://a.teamwork.com"}"#
        )
        .is_err());
    }
}
