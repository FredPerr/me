use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// The kind of a workspace-detection failure, as the frontend sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceDetectorErrorKind {
    /// The home directory could not be determined.
    HomeNotFound,
    /// The IDE's storage file does not exist yet.
    StorageNotFound,
    /// The storage file could not be read.
    StorageUnreadable,
    /// The storage file could not be parsed.
    StorageInvalid,
}

/// A failed workspace-detection command on the wire. `message` is a readable
/// sentence that may name a path on the user's own machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDetectorCommandError {
    pub kind: WorkspaceDetectorErrorKind,
    pub message: String,
}

#[derive(Deserialize)]
struct StorageJson {
    #[serde(rename = "windowsState")]
    windows_state: Option<WindowsState>,
}

#[derive(Deserialize)]
struct WindowsState {
    #[serde(rename = "lastActiveWindow")]
    last_active_window: Option<WindowEntry>,
    #[serde(rename = "openedWindows")]
    opened_windows: Option<Vec<WindowEntry>>,
}

#[derive(Deserialize)]
struct WindowEntry {
    folder: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_active_workspaces(
    ide_command: String,
) -> Result<Vec<String>, WorkspaceDetectorCommandError> {
    let storage_path = resolve_storage_path(&ide_command)?;

    let content = fs::read_to_string(&storage_path).map_err(|e| WorkspaceDetectorCommandError {
        kind: WorkspaceDetectorErrorKind::StorageUnreadable,
        message: format!("Failed to read storage file: {}", e),
    })?;

    let storage: StorageJson =
        serde_json::from_str(&content).map_err(|e| WorkspaceDetectorCommandError {
            kind: WorkspaceDetectorErrorKind::StorageInvalid,
            message: format!("Failed to parse storage: {}", e),
        })?;

    let workspaces = extract_workspace_paths(storage);
    Ok(workspaces)
}

fn resolve_storage_path(ide_command: &str) -> Result<PathBuf, WorkspaceDetectorCommandError> {
    let home = dirs::home_dir().ok_or(WorkspaceDetectorCommandError {
        kind: WorkspaceDetectorErrorKind::HomeNotFound,
        message: "Could not determine home directory".to_string(),
    })?;
    let app_name = ide_name_to_app_support_folder(ide_command);

    let path = home
        .join("Library/Application Support")
        .join(app_name)
        .join("User/globalStorage/storage.json");

    if path.exists() {
        Ok(path)
    } else {
        Err(WorkspaceDetectorCommandError {
            kind: WorkspaceDetectorErrorKind::StorageNotFound,
            message: format!("Storage file not found at: {:?}", path),
        })
    }
}

fn ide_name_to_app_support_folder(ide_command: &str) -> &str {
    match ide_command.to_lowercase().as_str() {
        "kiro" => "Kiro",
        "code" => "Code",
        "cursor" => "Cursor",
        "codium" | "vscodium" => "VSCodium",
        _ => "Code",
    }
}

fn extract_workspace_paths(storage: StorageJson) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();

    if let Some(state) = storage.windows_state {
        if let Some(window) = state.last_active_window {
            if let Some(folder) = window.folder {
                if let Some(path) = uri_to_path(&folder) {
                    paths.push(path);
                }
            }
        }

        if let Some(windows) = state.opened_windows {
            for window in windows {
                if let Some(folder) = window.folder {
                    if let Some(path) = uri_to_path(&folder) {
                        if !paths.contains(&path) {
                            paths.push(path);
                        }
                    }
                }
            }
        }
    }

    paths
}

fn uri_to_path(uri: &str) -> Option<String> {
    uri.strip_prefix("file://").map(percent_decode)
}

fn percent_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_storage_file_is_sent_as_the_storage_not_found_kind() {
        let error = WorkspaceDetectorCommandError {
            kind: WorkspaceDetectorErrorKind::StorageNotFound,
            message: "Storage file not found at: \"/x\"".to_string(),
        };

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!({
                "kind": "storageNotFound",
                "message": "Storage file not found at: \"/x\"",
            })
        );
    }

    #[test]
    fn maps_kiro_to_correct_folder() {
        assert_eq!(ide_name_to_app_support_folder("kiro"), "Kiro");
    }

    #[test]
    fn maps_code_to_correct_folder() {
        assert_eq!(ide_name_to_app_support_folder("code"), "Code");
    }

    #[test]
    fn maps_cursor_to_correct_folder() {
        assert_eq!(ide_name_to_app_support_folder("cursor"), "Cursor");
    }

    #[test]
    fn converts_file_uri_to_path() {
        let uri = "file:///Users/fred/Projects/my-app";

        let result = uri_to_path(uri);

        assert_eq!(result, Some("/Users/fred/Projects/my-app".to_string()));
    }

    #[test]
    fn decodes_percent_encoded_spaces() {
        let uri = "file:///Users/fred/My%20Projects/app";

        let result = uri_to_path(uri);

        assert_eq!(result, Some("/Users/fred/My Projects/app".to_string()));
    }

    #[test]
    fn returns_none_for_non_file_uri() {
        let uri = "https://example.com";

        let result = uri_to_path(uri);

        assert_eq!(result, None);
    }

    #[test]
    fn extracts_paths_from_storage_json() {
        let storage = StorageJson {
            windows_state: Some(WindowsState {
                last_active_window: Some(WindowEntry {
                    folder: Some("file:///Users/fred/Projects/app".to_string()),
                }),
                opened_windows: Some(vec![WindowEntry {
                    folder: Some("file:///Users/fred/Projects/lib".to_string()),
                }]),
            }),
        };

        let result = extract_workspace_paths(storage);

        assert_eq!(
            result,
            vec!["/Users/fred/Projects/app", "/Users/fred/Projects/lib",]
        );
    }

    #[test]
    fn returns_empty_when_no_windows_state() {
        let storage = StorageJson {
            windows_state: None,
        };

        let result = extract_workspace_paths(storage);

        assert!(result.is_empty());
    }

    #[test]
    fn deduplicates_active_and_opened_windows() {
        let storage = StorageJson {
            windows_state: Some(WindowsState {
                last_active_window: Some(WindowEntry {
                    folder: Some("file:///Users/fred/Projects/app".to_string()),
                }),
                opened_windows: Some(vec![WindowEntry {
                    folder: Some("file:///Users/fred/Projects/app".to_string()),
                }]),
            }),
        };

        let result = extract_workspace_paths(storage);

        assert_eq!(result, vec!["/Users/fred/Projects/app"]);
    }
}
