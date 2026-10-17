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
//! This ticket (#87) adds only the runner port, the agent-text extraction and
//! the composition adapter. The use cases and commands arrive in #88/#89.

pub mod domain;
