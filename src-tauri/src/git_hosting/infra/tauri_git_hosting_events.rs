use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::git_hosting::commands::HostKindView;
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::ports::GitHostingEvents;

/// Emitted as `git-hosting-changed` after signing in to or out of a Git host,
/// so every Git hosting view refetches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct GitHostingChanged {
    pub host: HostKindView,
}

pub struct TauriGitHostingEvents {
    app: AppHandle,
}

impl TauriGitHostingEvents {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl GitHostingEvents for TauriGitHostingEvents {
    fn changed(&self, host: HostKind) {
        let event = GitHostingChanged { host: host.into() };
        if let Err(error) = event.emit(&self.app) {
            eprintln!(
                "[git_hosting] git-hosting-changed for '{}' not emitted: {error}",
                host.as_str()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_named_git_hosting_changed_and_carries_the_host() {
        let event = GitHostingChanged {
            host: HostKindView::Github,
        };

        assert_eq!(GitHostingChanged::NAME, "git-hosting-changed");
        assert_eq!(
            serde_json::to_value(event).unwrap(),
            serde_json::json!({ "host": "github" })
        );
    }
}
