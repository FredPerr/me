//! AI CLI session orchestration.
//!
//! Spawns AI CLI tools (Kiro today, others via [`adapter`]) as child processes
//! with piped stdout/stderr, tracks them in an in-memory [`registry`], streams
//! their output to the frontend over Tauri events line-by-line, and terminates
//! them by process group on request.
//!
//! No Tauri command is exposed: AI sessions are driven only by Rust use cases
//! (agent runs and AI assists) through [`process::spawn_with_sink`] and
//! [`process::kill_session`]. The frontend no longer spawns a session (#90).

pub mod adapter;
pub mod kiro;
pub mod process;
pub mod registry;

use registry::SessionStatus;
use serde::Serialize;

/// Event name carrying a line of output for a session.
pub const EVENT_OUTPUT: &str = "ai-session://output";
/// Event name carrying a lifecycle status change for a session.
pub const EVENT_STATUS: &str = "ai-session://status";

/// Which stream a line of output came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// A single line of streamed output, emitted on [`EVENT_OUTPUT`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEvent {
    pub session_id: String,
    pub stream: OutputStream,
    pub line: String,
}

/// A lifecycle transition, emitted on [`EVENT_STATUS`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusEvent {
    pub session_id: String,
    pub adapter_id: String,
    pub status: SessionStatus,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    /// Present when `status` is `failed`, describing what went wrong.
    pub error: Option<String>,
}

/// The descriptor of a started session, returned by the spawn adapters.
#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpawnedSession {
    pub session_id: String,
    pub adapter_id: String,
    pub pid: Option<u32>,
    pub working_directory: String,
}
