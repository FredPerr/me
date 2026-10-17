//! AI-assist domain: the runner port and the pure agent-text reconstruction.
//! Depends only on `std`, `tokio` (for the cancel signal and async trait) and
//! `serde_json` (to parse ACP stream lines). It names no other bounded context.

pub mod agent_text;
pub mod ai_assist_runner;
