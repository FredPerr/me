//! AI CLI session orchestration.
//!
//! Spawns AI CLI tools (Kiro today, others via [`adapter`]) as child processes
//! with piped stdout/stderr, tracks them in an in-memory [`registry`], streams
//! their output to the frontend over Tauri events line-by-line, and terminates
//! them by process group on request.
//!
//! The typed Tauri commands exposed are [`spawn_ai_session`] and
//! [`kill_ai_session`], both driving the frontend's AI-assist drafts.

pub mod adapter;
pub mod kiro;
pub mod process;
pub mod registry;

use adapter::SpawnRequest;
use registry::{SessionRegistry, SessionStatus};
use serde::{Deserialize, Serialize};
use tauri::State;

/// The kind of an AI-session command failure, as the frontend sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum AiSessionErrorKind {
    /// No adapter is registered for the requested id.
    UnknownAdapter,
    /// The prompt was empty.
    EmptyPrompt,
    /// The session could not be spawned (binary not found, spawn failed).
    SpawnFailed,
    /// The session could not be killed.
    KillFailed,
}

/// A failed AI-session command on the wire. `message` is a readable sentence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AiSessionCommandError {
    pub kind: AiSessionErrorKind,
    pub message: String,
}

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

/// Parameters for spawning a session, sent from the frontend (camelCase).
#[derive(Debug, Deserialize, specta::Type)]
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
    /// Optional prior conversation id to resume, so a follow-up prompt keeps the
    /// earlier context (Kiro's `--resume-id`).
    pub resume_id: Option<String>,
    /// Optional login shell used to resolve the binary via the user's PATH.
    pub shell: Option<String>,
    /// Extra environment variables (e.g. the headless API key).
    #[serde(default)]
    pub environment: Vec<EnvVar>,
    /// Run tools without per-action confirmation. Defaults to false when
    /// absent; the frontend sends true for headless runs, which rely on a
    /// workspace permission policy for their safety floor.
    #[serde(default)]
    pub trust_all_tools: bool,
}

/// A single environment variable pair from the frontend.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

/// The result returned to the caller when a session is accepted and started.
#[derive(Debug, Serialize, specta::Type)]
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
#[specta::specta]
pub async fn spawn_ai_session(
    app: tauri::AppHandle,
    registry: State<'_, SessionRegistry>,
    options: SpawnOptions,
) -> Result<SpawnedSession, AiSessionCommandError> {
    let adapter = registry
        .adapters()
        .get(&options.adapter_id)
        .ok_or_else(|| AiSessionCommandError {
            kind: AiSessionErrorKind::UnknownAdapter,
            message: format!("Unknown AI CLI adapter '{}'", options.adapter_id),
        })?;

    if options.prompt.trim().is_empty() {
        return Err(AiSessionCommandError {
            kind: AiSessionErrorKind::EmptyPrompt,
            message: "Prompt must not be empty".to_string(),
        });
    }

    let request = SpawnRequest {
        prompt: options.prompt,
        working_directory: options.working_directory,
        agent: options.agent,
        resume_id: options.resume_id,
        shell: options.shell,
        environment: options
            .environment
            .into_iter()
            .map(|pair| (pair.key, pair.value))
            .collect(),
        trust_all_tools: options.trust_all_tools,
    };

    let spec = adapter
        .build_spec(&request, options.command.as_deref())
        .map_err(|message| AiSessionCommandError {
            kind: AiSessionErrorKind::SpawnFailed,
            message,
        })?;

    process::spawn(app, &registry, options.adapter_id, spec)
        .await
        .map_err(|message| AiSessionCommandError {
            kind: AiSessionErrorKind::SpawnFailed,
            message,
        })
}

/// Terminates a running session by killing its entire process group.
///
/// A kill is flagged in the registry first so the exit is reported as `killed`
/// rather than `completed`/`failed`, then the OS process group is signalled.
#[tauri::command]
#[specta::specta]
pub async fn kill_ai_session(
    registry: State<'_, SessionRegistry>,
    session_id: String,
) -> Result<(), AiSessionCommandError> {
    process::kill_session(&registry, &session_id).map_err(|message| AiSessionCommandError {
        kind: AiSessionErrorKind::KillFailed,
        message,
    })
}
