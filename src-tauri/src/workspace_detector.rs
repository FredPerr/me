use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

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
pub async fn get_active_workspaces(ide_command: String) -> Result<Vec<String>, String> {
    let storage_path = resolve_storage_path(&ide_command)?;

    let content = fs::read_to_string(&storage_path)
        .map_err(|e| format!("Failed to read storage file: {}", e))?;

    let storage: StorageJson =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse storage: {}", e))?;

    let workspaces = extract_workspace_paths(storage);
    Ok(workspaces)
}

fn resolve_storage_path(ide_command: &str) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    let app_name = ide_name_to_app_support_folder(ide_command);

    let path = home
        .join("Library/Application Support")
        .join(app_name)
        .join("User/globalStorage/storage.json");

    if path.exists() {
        Ok(path)
    } else {
        Err(format!("Storage file not found at: {:?}", path))
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
    uri.strip_prefix("file://").map(|p| {
        percent_decode(p)
    })
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
                opened_windows: Some(vec![
                    WindowEntry { folder: Some("file:///Users/fred/Projects/lib".to_string()) },
                ]),
            }),
        };

        let result = extract_workspace_paths(storage);

        assert_eq!(result, vec![
            "/Users/fred/Projects/app",
            "/Users/fred/Projects/lib",
        ]);
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
                opened_windows: Some(vec![
                    WindowEntry { folder: Some("file:///Users/fred/Projects/app".to_string()) },
                ]),
            }),
        };

        let result = extract_workspace_paths(storage);

        assert_eq!(result, vec!["/Users/fred/Projects/app"]);
    }
}
