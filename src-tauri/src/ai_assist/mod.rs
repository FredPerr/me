//! AI assists: read-only suggestions produced by running an AI tool to
//! completion and reading its answer back (spec #85).
//!
//! An AI assist is not a streaming agent run. It builds a prompt, runs the tool
//! read-only to completion, reconstructs the agent's final answer from the
//! output, and returns it as a single result — nothing is streamed, nothing is
//! persisted, and the run may touch no files.
//!
//! The module is layered like the others and grows over #88/#89:
//!   - `domain`: the [`AiAssistRunner`](domain::ai_assist_runner::AiAssistRunner)
//!     port and its spec/error/cancel types, plus the pure agent-text
//!     reconstruction ported from the frontend.
//!   - the composition root (`crate::composition::ai_session_ai_assist_runner`)
//!     implements the runner over `ai_session`; neither module names the other.
//!
//! #88 adds the `application` layer (`AiAssistService::suggest_contexts`), the
//! ports it reads through, and the typed commands; #89 adds the pull-request
//! suggestion alongside it.

pub mod application;
pub mod commands;
pub mod domain;

use std::sync::Arc;

use application::suggest_contexts::AiAssistService;

/// The AI-assist service, held in Tauri state. Built in the composition root
/// from the runner adapter and the settings/workspace/temp ports.
pub struct AiAssist {
    pub service: Arc<AiAssistService>,
}
