//! The `AgentRunner` port: how `workspace` starts, observes and stops one run
//! of an AI coding tool without knowing which process runner backs it. A
//! crate-level composition adapter implements it over `ai_session`; `workspace`
//! never names that module.
//!
//! The run's events are delivered to a sink the caller provides (choice 2A),
//! so the `run_agent` use case (#80) can consume them on a background task to
//! persist lines and emit its own Tauri event as they arrive. Starting a run
//! returns a [`RunHandle`] that `stop` later uses to terminate it.

use std::fmt;

use async_trait::async_trait;
use tokio::sync::mpsc;

/// What to run: the tool, the folder it runs in, the prompt, and an optional
/// prior conversation id to resume so a follow-up keeps the earlier context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunSpec {
    /// The AI tool to run, by its adapter id (e.g. `"kiro"`).
    pub tool_id: String,
    /// Directory the tool runs in (the context's `ide_path`).
    pub working_directory: String,
    /// The instruction to run headlessly.
    pub prompt: String,
    /// A prior conversation to resume, when the context already has one.
    pub resume_id: Option<String>,
}

/// A handle to a started run, used to stop it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHandle {
    pub run_id: String,
}

/// How a run ended, independent of the underlying process runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunFinalState {
    /// The tool exited on its own successfully.
    Finished,
    /// The tool failed to start or exited with a non-success status.
    Failed,
    /// The run was stopped on request.
    Stopped,
}

/// An event in a run's life, delivered to the sink in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRunEvent {
    /// The run has started and is now producing output.
    Started { run_id: String },
    /// One raw line of the tool's output, as the runner emitted it.
    Line { text: String },
    /// A conversation id the tool surfaced in its output, so a follow-up run
    /// can resume it. Emitted at most once per run, when first seen.
    ConversationId { id: String },
    /// The run reached a terminal state; no further events follow.
    Ended {
        final_state: RunFinalState,
        exit_code: Option<i32>,
    },
}

/// The sink a run's events are delivered to. A channel so the use case can
/// consume events on a background task while the runner produces them.
pub type AgentRunEventSink = mpsc::UnboundedSender<AgentRunEvent>;

/// Why starting or stopping a run failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRunError {
    /// No tool is registered under the requested id.
    UnknownTool { tool_id: String },
    /// The prompt was empty or only whitespace.
    EmptyPrompt,
    /// No run is tracked under the handle given to `stop`.
    RunNotFound { run_id: String },
    /// The run could not be started or stopped (process-level failure).
    Failed { reason: String },
}

impl fmt::Display for AgentRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool { tool_id } => {
                write!(formatter, "Unknown agent tool '{tool_id}'")
            }
            Self::EmptyPrompt => write!(formatter, "The prompt must not be empty"),
            Self::RunNotFound { run_id } => {
                write!(formatter, "No agent run with id '{run_id}'")
            }
            Self::Failed { reason } => write!(formatter, "The agent run failed: {reason}"),
        }
    }
}

impl std::error::Error for AgentRunError {}

/// Starts, observes and stops an agent run. Implemented by a composition
/// adapter over the process runner; the use case depends only on this trait.
#[async_trait]
pub trait AgentRunner: Send + Sync {
    /// Starts the run described by `spec`, delivering its events to `sink`, and
    /// returns a handle to stop it with. The `Started` event is delivered to
    /// the sink; the handle lets the caller stop the run later.
    async fn start(
        &self,
        spec: AgentRunSpec,
        sink: AgentRunEventSink,
    ) -> Result<RunHandle, AgentRunError>;

    /// Stops a started run. Its sink then receives an `Ended` event with a
    /// `Stopped` final state.
    async fn stop(&self, handle: &RunHandle) -> Result<(), AgentRunError>;
}
