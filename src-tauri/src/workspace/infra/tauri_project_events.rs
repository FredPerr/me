use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::workspace::domain::ports::ProjectEvents;
use crate::workspace::domain::project_tag::ProjectTag;

/// Emitted as `project-changed` whenever a Project is saved, so every open
/// view refetches it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChanged {
    pub tag: String,
}

pub struct TauriProjectEvents {
    app: AppHandle,
}

impl TauriProjectEvents {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl ProjectEvents for TauriProjectEvents {
    fn project_changed(&self, tag: &ProjectTag) {
        let event = ProjectChanged {
            tag: tag.to_string(),
        };
        if let Err(error) = event.emit(&self.app) {
            eprintln!("[workspace] project-changed for '{tag}' not emitted: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_named_project_changed_and_carries_the_tag() {
        let event = ProjectChanged {
            tag: "my-project".to_string(),
        };

        assert_eq!(ProjectChanged::NAME, "project-changed");
        assert_eq!(
            serde_json::to_value(event).unwrap(),
            serde_json::json!({ "tag": "my-project" })
        );
    }
}
