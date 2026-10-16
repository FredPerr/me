//! Typed Tauri commands of the Settings context and their wire types.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use super::application::service::SettingsService;
use super::domain::app_settings::AppSettings;
use super::domain::ports::SettingsStoreUnavailable;
use super::Settings;

/// The app-wide settings on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsView {
    pub assign_me_to_tasks: bool,
}

impl From<AppSettings> for AppSettingsView {
    fn from(settings: AppSettings) -> Self {
        Self {
            assign_me_to_tasks: settings.assign_me_to_tasks,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SettingsErrorKind {
    Storage,
}

/// A failed Settings command on the wire. `message` is a fixed English sentence
/// per kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsCommandError {
    pub kind: SettingsErrorKind,
    pub message: String,
}

impl From<SettingsStoreUnavailable> for SettingsCommandError {
    fn from(_: SettingsStoreUnavailable) -> Self {
        Self {
            kind: SettingsErrorKind::Storage,
            message: "The settings could not be saved.".to_string(),
        }
    }
}

fn service(settings: &State<'_, Settings>) -> Arc<SettingsService> {
    settings.service.clone()
}

/// The current settings, falling back to the defaults when none are stored or
/// the store cannot be read.
#[tauri::command]
#[specta::specta]
pub async fn settings_get(settings: State<'_, Settings>) -> Result<AppSettingsView, ()> {
    let app_settings = service(&settings).get().unwrap_or_default();
    Ok(app_settings.into())
}

/// Sets whether "Assign me to these tasks" is checked by default and returns
/// the updated settings.
#[tauri::command]
#[specta::specta]
pub async fn settings_set_assign_me_to_tasks(
    settings: State<'_, Settings>,
    value: bool,
) -> Result<AppSettingsView, SettingsCommandError> {
    let updated = service(&settings).set_assign_me_to_tasks(value)?;
    Ok(updated.into())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_storage_failure_is_sent_as_the_storage_kind() {
        let error = SettingsCommandError::from(SettingsStoreUnavailable);

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({
                "kind": "storage",
                "message": "The settings could not be saved.",
            })
        );
    }

    #[test]
    fn the_settings_are_sent_in_camel_case() {
        let view = AppSettingsView::from(AppSettings {
            assign_me_to_tasks: true,
        });

        assert_eq!(
            serde_json::to_value(view).unwrap(),
            json!({ "assignMeToTasks": true })
        );
    }
}
