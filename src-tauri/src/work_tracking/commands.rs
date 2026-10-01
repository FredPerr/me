//! Tauri command surface of the Work Tracking context.
use serde::{Deserialize, Serialize};
use tauri::State;

use super::application::service::{ConnectionView, SaveConnectionCommand, WorkTrackingService};
use super::domain::error::{InputField, InvalidInputReason, WorkTrackingError};
use super::domain::page::Page;
use super::domain::work_item::WorkItem;
use super::domain::work_item_group::WorkItemGroup;
use super::domain::work_project::WorkProject;

/// Structured wire error. `message` is a fixed English sentence per kind and
/// never contains provider or credential data.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    kind: &'static str,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
}

impl CommandError {
    fn new(kind: &'static str, message: &'static str) -> Self {
        Self {
            kind,
            message,
            field: None,
            reason: None,
            retry_after_seconds: None,
            status: None,
        }
    }
}

fn field_name(field: InputField) -> &'static str {
    match field {
        InputField::ProviderKind => "providerKind",
        InputField::BaseUrl => "baseUrl",
        InputField::ApiKey => "apiKey",
        InputField::DisplayName => "displayName",
        InputField::ConnectionId => "connectionId",
        InputField::ProjectId => "projectId",
        InputField::GroupId => "groupId",
        InputField::Cursor => "cursor",
        InputField::PageSize => "pageSize",
    }
}

fn reason_name(reason: InvalidInputReason) -> &'static str {
    match reason {
        InvalidInputReason::Required => "required",
        InvalidInputReason::TooLong => "tooLong",
        InvalidInputReason::InvalidFormat => "invalidFormat",
        InvalidInputReason::HttpsRequired => "httpsRequired",
        InvalidInputReason::UnsupportedComponent => "unsupportedComponent",
        InvalidInputReason::OutOfRange => "outOfRange",
        InvalidInputReason::Unsupported => "unsupported",
    }
}

impl From<WorkTrackingError> for CommandError {
    fn from(error: WorkTrackingError) -> Self {
        match error {
            WorkTrackingError::NotConfigured => {
                Self::new("notConfigured", "The connection is not configured.")
            }
            WorkTrackingError::InvalidInput { field, reason } => Self {
                field: Some(field_name(field)),
                reason: Some(reason_name(reason)),
                ..Self::new("invalidInput", "The input is invalid.")
            },
            WorkTrackingError::Unauthorized => {
                Self::new("unauthorized", "The provider rejected the credentials.")
            }
            WorkTrackingError::Forbidden => {
                Self::new("forbidden", "Access to this resource is forbidden.")
            }
            WorkTrackingError::NotFound => {
                Self::new("notFound", "The requested resource was not found.")
            }
            WorkTrackingError::RateLimited {
                retry_after_seconds,
            } => Self {
                retry_after_seconds,
                ..Self::new("rateLimited", "The provider rate limit was reached.")
            },
            WorkTrackingError::Timeout => {
                Self::new("timeout", "The request to the provider timed out.")
            }
            WorkTrackingError::Network => {
                Self::new("network", "The provider could not be reached.")
            }
            WorkTrackingError::ProviderError { status } => Self {
                status,
                ..Self::new(
                    "providerError",
                    "The provider returned an unexpected error.",
                )
            },
            WorkTrackingError::InvalidResponse => Self::new(
                "invalidResponse",
                "The provider returned an invalid response.",
            ),
            WorkTrackingError::StorageError => {
                Self::new("storageError", "Local storage could not be accessed.")
            }
        }
    }
}

// No Debug derive: holds the raw API key. Fields are moved uninspected into
// `SaveConnectionCommand`; the service validates them. The type is `pub` only
// because `#[tauri::command]` requires it; its fields stay private.
#[derive(Deserialize)]
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

#[tauri::command]
pub async fn work_tracking_list_connections(
    state: State<'_, WorkTrackingService>,
) -> Result<Vec<ConnectionView>, CommandError> {
    Ok(state.list_connections()?)
}

#[tauri::command]
pub async fn work_tracking_save_connection(
    state: State<'_, WorkTrackingService>,
    input: SaveConnectionInput,
) -> Result<ConnectionView, CommandError> {
    Ok(state.save_connection(input.into()).await?)
}

#[tauri::command]
pub async fn work_tracking_remove_connection(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
) -> Result<(), CommandError> {
    Ok(state.remove_connection(&connection_id)?)
}

#[tauri::command]
pub async fn work_tracking_list_projects(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
    cursor: Option<String>,
) -> Result<Page<WorkProject>, CommandError> {
    Ok(state
        .list_projects(&connection_id, cursor.as_deref())
        .await?)
}

#[tauri::command]
pub async fn work_tracking_list_groups(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
    project_id: String,
    cursor: Option<String>,
) -> Result<Page<WorkItemGroup>, CommandError> {
    Ok(state
        .list_groups(&connection_id, &project_id, cursor.as_deref())
        .await?)
}

#[tauri::command]
pub async fn work_tracking_list_items(
    state: State<'_, WorkTrackingService>,
    connection_id: String,
    project_id: String,
    group_id: String,
    cursor: Option<String>,
) -> Result<Page<WorkItem>, CommandError> {
    Ok(state
        .list_items(&connection_id, &project_id, &group_id, cursor.as_deref())
        .await?)
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    fn wire(error: WorkTrackingError) -> Value {
        serde_json::to_value(CommandError::from(error)).unwrap()
    }

    #[test]
    fn unauthorized_has_only_kind_and_message() {
        assert_eq!(
            wire(WorkTrackingError::Unauthorized),
            json!({ "kind": "unauthorized", "message": "The provider rejected the credentials." })
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
        assert!(value.get("status").is_none());
        assert!(value.get("retryAfterSeconds").is_none());
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
        assert!(limited_without_delay.get("retryAfterSeconds").is_none());
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
            assert_eq!(value.as_object().unwrap().len(), 2, "{value}");
        }
    }

    #[test]
    fn every_field_and_reason_has_a_camel_case_name() {
        assert_eq!(field_name(InputField::ProviderKind), "providerKind");
        assert_eq!(field_name(InputField::ApiKey), "apiKey");
        assert_eq!(field_name(InputField::DisplayName), "displayName");
        assert_eq!(field_name(InputField::ConnectionId), "connectionId");
        assert_eq!(field_name(InputField::ProjectId), "projectId");
        assert_eq!(field_name(InputField::GroupId), "groupId");
        assert_eq!(field_name(InputField::Cursor), "cursor");
        assert_eq!(field_name(InputField::PageSize), "pageSize");
        assert_eq!(reason_name(InvalidInputReason::Required), "required");
        assert_eq!(reason_name(InvalidInputReason::TooLong), "tooLong");
        assert_eq!(
            reason_name(InvalidInputReason::InvalidFormat),
            "invalidFormat"
        );
        assert_eq!(
            reason_name(InvalidInputReason::UnsupportedComponent),
            "unsupportedComponent"
        );
        assert_eq!(reason_name(InvalidInputReason::OutOfRange), "outOfRange");
        assert_eq!(reason_name(InvalidInputReason::Unsupported), "unsupported");
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
