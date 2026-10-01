//! AI CLI session orchestration.
//!
//! Spawns AI CLI tools (Kiro today, others via [`adapter`]) as child processes
//! with piped stdout/stderr, tracks them in an in-memory [`registry`], streams
//! their output to the frontend over Tauri events line-by-line, and terminates
//! them by process group on request. Every lifecycle transition is also pushed
//! to the shared local MCP server via [`mcp_sync`] so the IDE stays in sync.
//!
//! The three Tauri commands exposed are [`spawn_ai_session`], [`kill_ai_session`]
//! and [`list_ai_adapters`]; [`list_ai_sessions`] reports currently tracked ids.

pub mod adapter;
pub mod kiro;
pub mod mcp_sync;
pub mod registry;

mod process;

use adapter::{AdapterInfo, SpawnRequest};
use registry::{SessionRegistry, SessionStatus};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Event name carrying a line of output for a session.
pub const EVENT_OUTPUT: &str = "ai-session://output";
/// Event name carrying a lifecycle status change for a session.
pub const EVENT_STATUS: &str = "ai-session://status";

/// Which stream a line of output came from.
#[derive(Debug, Clone, Copy, Serialize)]
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

/// Parameters for spawning a session, sent from the frontend (camelCase).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnOptions {
    /// Adapter id selecting the CLI to run (e.g. `"kiro"`).
    pub adapter_id: String,
    /// Natural-language prompt / instruction to run headlessly.
    pub prompt: String,
    /// Directory the CLI runs in (typically a context worktree path).
    pub working_directory: String,
    /// Optional explicit binary/path override; falls back to the adapter default.
    pub command: Option<String>,
    /// Optional named agent/profile when supported by the CLI.
    pub agent: Option<String>,
    /// Optional login shell used to resolve the binary via the user's PATH.
    pub shell: Option<String>,
    /// Extra environment variables (e.g. the headless API key).
    #[serde(default)]
    pub environment: Vec<EnvVar>,
}

/// A single environment variable pair from the frontend.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

/// The result returned to the caller when a session is accepted and started.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnedSession {
    pub session_id: String,
    pub adapter_id: String,
    pub pid: Option<u32>,
    pub working_directory: String,
}

/// Spawns a new AI CLI session and begins streaming its output.
///
/// Resolves the chosen adapter, builds a headless spawn spec, launches the
/// child with piped stdio in its own process group, registers it, and starts
/// background tasks that forward stdout/stderr lines to the frontend and watch
/// for exit. Returns immediately with the session id and pid.
#[tauri::command]
pub async fn spawn_ai_session(
    app: tauri::AppHandle,
    registry: State<'_, SessionRegistry>,
    options: SpawnOptions,
) -> Result<SpawnedSession, String> {
    let adapter = registry
        .adapters()
        .get(&options.adapter_id)
        .ok_or_else(|| format!("Unknown AI CLI adapter '{}'", options.adapter_id))?;

    if options.prompt.trim().is_empty() {
        return Err("Prompt must not be empty".to_string());
    }

    let request = SpawnRequest {
        prompt: options.prompt,
        working_directory: options.working_directory,
        agent: options.agent,
        shell: options.shell,
        environment: options
            .environment
            .into_iter()
            .map(|pair| (pair.key, pair.value))
            .collect(),
    };

    let spec = adapter.build_spec(&request, options.command.as_deref())?;

    process::spawn(app, &registry, options.adapter_id, spec).await
}

/// Terminates a running session by killing its entire process group.
///
/// A kill is flagged in the registry first so the exit is reported as `killed`
/// rather than `completed`/`failed`, then the OS process group is signalled.
#[tauri::command]
pub async fn kill_ai_session(
    registry: State<'_, SessionRegistry>,
    session_id: String,
) -> Result<(), String> {
    let pid = registry
        .mark_kill_requested(&session_id)
        .or_else(|| registry.pid_of(&session_id))
        .ok_or_else(|| format!("No running session with id '{}'", session_id))?;

    process::kill_process_group(pid)
}

/// Lists the ids of sessions currently tracked as running.
#[tauri::command]
pub async fn list_ai_sessions(registry: State<'_, SessionRegistry>) -> Result<Vec<String>, String> {
    Ok(registry.running_ids())
}

/// Lists the AI CLI adapters this build supports, for populating the UI.
#[tauri::command]
pub async fn list_ai_adapters(
    registry: State<'_, SessionRegistry>,
) -> Result<Vec<AdapterInfo>, String> {
    Ok(registry.adapters().list())
}
