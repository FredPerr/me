use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::workspace::domain::ports::WorkbenchEvents;

/// Emitted as `workbench-changed` whenever the Workbench is saved, so every
/// open window refetches the arrangement. There is one Workbench per app, so
/// the event carries no payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchChanged {}

pub struct TauriWorkbenchEvents {
    app: AppHandle,
}

impl TauriWorkbenchEvents {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl WorkbenchEvents for TauriWorkbenchEvents {
    fn workbench_changed(&self) {
        let event = WorkbenchChanged {};
        if let Err(error) = event.emit(&self.app) {
            eprintln!("[workspace] workbench-changed not emitted: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_named_workbench_changed() {
        assert_eq!(WorkbenchChanged::NAME, "workbench-changed");
        assert_eq!(
            serde_json::to_value(WorkbenchChanged {}).unwrap(),
            serde_json::json!({})
        );
    }
}
