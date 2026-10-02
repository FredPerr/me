//! In-memory registry of running AI CLI sessions.
//!
//! The registry is the single source of truth for live child processes. It
//! maps a unique `session_id` to the information needed to observe and kill the
//! process — its OS pid and a handle used to terminate it. Terminated or
//! finished sessions are removed from the registry; their final state lives in
//! the frontend (and the MCP server) rather than here.
//!
//! Access is guarded by a `Mutex` and the whole registry is shared as Tauri
//! managed state, matching the `Arc`/`Mutex` shared-state style already used in
//! `git.rs`.

use crate::ai_session::adapter::AdapterRegistry;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;

/// The lifecycle state of a session, surfaced to the frontend as a tagged union
/// (mirrors the `DevicePollResult` pattern in `git_provider::github`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionStatus {
    /// The process is spawning or actively running.
    Running,
    /// The process exited on its own.
    Completed,
    /// The process failed to spawn or exited with a non-success status.
    Failed,
    /// The process was terminated by the user.
    Killed,
}

/// Bookkeeping for a live process tracked by the registry.
///
/// The `Child` handle itself is owned by the exit-watcher task, not the
/// registry, so that awaiting exit never blocks registry access. The registry
/// retains only what `kill` needs (the pid) plus the kill-requested flag used
/// to classify the terminal state.
pub struct RunningSession {
    pub session_id: String,
    pub pid: Option<u32>,
    /// When true, a kill was requested; used to distinguish an intentional
    /// termination from a natural exit when the wait loop observes the exit.
    pub kill_requested: bool,
}

/// Thread-safe collection of running sessions plus the available adapters.
pub struct SessionRegistry {
    sessions: Mutex<HashMap<String, RunningSession>>,
    adapters: AdapterRegistry,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            adapters: AdapterRegistry::builtin(),
        }
    }

    pub fn adapters(&self) -> &AdapterRegistry {
        &self.adapters
    }

    /// Inserts a newly spawned session.
    pub fn insert(&self, session: RunningSession) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(session.session_id.clone(), session);
        }
    }

    /// Removes and returns a session, e.g. after it exits.
    pub fn remove(&self, session_id: &str) -> Option<RunningSession> {
        self.sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.remove(session_id))
    }

    /// Returns the pid of a tracked session, if present.
    pub fn pid_of(&self, session_id: &str) -> Option<u32> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(session_id).and_then(|session| session.pid))
    }

    /// Marks a session as having a kill requested and returns its pid.
    pub fn mark_kill_requested(&self, session_id: &str) -> Option<u32> {
        let mut sessions = self.sessions.lock().ok()?;
        let session = sessions.get_mut(session_id)?;
        session.kill_requested = true;
        session.pid
    }

    /// Lists the ids of currently running sessions.
    pub fn running_ids(&self) -> Vec<String> {
        self.sessions
            .lock()
            .map(|sessions| sessions.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registry_has_no_running_sessions() {
        let registry = SessionRegistry::new();

        assert!(registry.running_ids().is_empty());
        assert!(registry.pid_of("missing").is_none());
    }

    #[test]
    fn builtin_adapters_include_kiro() {
        let registry = SessionRegistry::new();

        assert!(registry.adapters().get("kiro").is_some());
    }
}
