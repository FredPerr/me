//! Synchronizes session state with the shared local MCP server.
//!
//! Both this GUI and the Kiro IDE talk to a shared local MCP server. When a
//! session changes state (`running` → `completed`/`failed`/`killed`), we POST
//! the new state so the IDE and any other MCP client stay in sync with what the
//! GUI is doing.
//!
//! Design notes:
//! - The endpoint is configurable (constructor arg), defaulting to a localhost
//!   URL and overridable via the `ME_MCP_SYNC_URL` environment variable. This
//!   keeps the integration flexible as the MCP surface evolves.
//! - Sync is best-effort: the MCP server being down or slow must never break
//!   session spawning or killing. Every failure is swallowed into a logged
//!   warning rather than propagated, so the GUI keeps working standalone.

use crate::ai_session::registry::SessionStatus;
use serde::Serialize;
use std::time::Duration;

const DEFAULT_SYNC_URL: &str = "http://127.0.0.1:7000/sessions";
const SYNC_URL_ENV: &str = "ME_MCP_SYNC_URL";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// A single state transition reported to the MCP server. Serialized as camelCase
/// JSON for a conventional HTTP/JSON MCP bridge.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateUpdate {
    pub session_id: String,
    pub adapter_id: String,
    pub status: String,
    pub working_directory: String,
    /// Process id while running; absent once the process has exited.
    pub pid: Option<u32>,
    /// Process exit code, when the session has finished.
    pub exit_code: Option<i32>,
}

/// Best-effort client that pushes session state to the shared MCP server.
#[derive(Clone)]
pub struct McpSync {
    endpoint: String,
    client: reqwest::Client,
}

impl McpSync {
    /// Builds a client, resolving the endpoint from the environment or falling
    /// back to the localhost default. Returns `None` only if the HTTP client
    /// cannot be constructed, in which case callers skip syncing.
    pub fn from_environment() -> Option<Self> {
        let endpoint = std::env::var(SYNC_URL_ENV).unwrap_or_else(|_| DEFAULT_SYNC_URL.to_string());
        Self::new(endpoint)
    }

    pub fn new(endpoint: impl Into<String>) -> Option<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .ok()?;
        Some(Self {
            endpoint: endpoint.into(),
            client,
        })
    }

    /// Reports a state transition. Never fails the caller: network/HTTP errors
    /// are logged and swallowed so GUI operations stay unaffected.
    pub async fn report(&self, update: SessionStateUpdate) {
        let result = self.client.post(&self.endpoint).json(&update).send().await;

        match result {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => {
                eprintln!(
                    "[mcp_sync] MCP server returned {} for session {} ({})",
                    response.status(),
                    update.session_id,
                    update.status
                );
            }
            Err(error) => {
                eprintln!(
                    "[mcp_sync] could not reach MCP server at {}: {} (session {} state '{}' not synced)",
                    self.endpoint, error, update.session_id, update.status
                );
            }
        }
    }
}

/// Convenience constructor for an update from the common pieces.
pub fn build_update(
    session_id: &str,
    adapter_id: &str,
    status: SessionStatus,
    working_directory: &str,
    pid: Option<u32>,
    exit_code: Option<i32>,
) -> SessionStateUpdate {
    SessionStateUpdate {
        session_id: session_id.to_string(),
        adapter_id: adapter_id.to_string(),
        status: status.as_label().to_string(),
        working_directory: working_directory.to_string(),
        pid,
        exit_code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_update_maps_status_to_label() {
        let update = build_update(
            "session-1",
            "kiro",
            SessionStatus::Killed,
            "/tmp/project",
            Some(42),
            None,
        );

        assert_eq!(update.session_id, "session-1");
        assert_eq!(update.adapter_id, "kiro");
        assert_eq!(update.status, "killed");
        assert_eq!(update.pid, Some(42));
        assert_eq!(update.exit_code, None);
    }

    #[test]
    fn client_builds_with_default_endpoint() {
        let sync = McpSync::new(DEFAULT_SYNC_URL);

        assert!(sync.is_some());
    }
}
