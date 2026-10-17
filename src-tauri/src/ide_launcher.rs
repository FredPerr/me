use crate::binary_resolver;
use serde::Serialize;
use std::process::Command;

/// The kind of an IDE-launch failure, as the frontend sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum IdeLauncherErrorKind {
    /// The IDE command could not be located on the system.
    CommandNotFound,
    /// The resolved binary could not be launched.
    LaunchFailed,
}

/// A failed IDE-launch command on the wire. `message` is a readable sentence
/// that may name the user's own IDE command and project path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IdeLauncherCommandError {
    pub kind: IdeLauncherErrorKind,
    pub message: String,
}

/// Launches the configured IDE on a project path. Typed tauri-specta command;
/// the frontend calls it as `open_in_ide`.
#[tauri::command]
#[specta::specta]
pub async fn open_in_ide(
    command: String,
    path: String,
    shell: Option<String>,
) -> Result<(), IdeLauncherCommandError> {
    launch(&command, &path, shell.as_deref())
}

/// Resolves an IDE command to an absolute binary path and launches it with the
/// given project path.
///
/// GUI apps launched from Finder do not inherit the user's interactive shell
/// PATH, so a bare command name like `kiro` cannot be found. This resolves the
/// command to an absolute path first (via [`binary_resolver`]), then spawns the
/// binary directly without a shell (avoiding shell injection on the project
/// path).
pub fn launch(
    command: &str,
    path: &str,
    shell: Option<&str>,
) -> Result<(), IdeLauncherCommandError> {
    let binary =
        binary_resolver::resolve(command, shell).ok_or_else(|| IdeLauncherCommandError {
            kind: IdeLauncherErrorKind::CommandNotFound,
            message: format!("Could not locate IDE command '{}' on this system", command),
        })?;

    Command::new(&binary)
        .arg(path)
        .spawn()
        .map_err(|error| IdeLauncherCommandError {
            kind: IdeLauncherErrorKind::LaunchFailed,
            message: format!("Failed to launch '{}': {}", binary.display(), error),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_missing_command_is_sent_as_the_command_not_found_kind() {
        let error = IdeLauncherCommandError {
            kind: IdeLauncherErrorKind::CommandNotFound,
            message: "Could not locate IDE command 'nope' on this system".to_string(),
        };

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({
                "kind": "commandNotFound",
                "message": "Could not locate IDE command 'nope' on this system",
            })
        );
    }
}
