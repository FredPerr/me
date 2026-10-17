//! AI-assist domain: the runner port, the pure agent-text reconstruction, the
//! bulk-context prompt and parser, the context-draft types, and the ports the
//! `suggest_contexts` use case reads through. Depends only on `std`, `tokio`
//! (cancel signal, async trait) and `serde_json` (ACP lines, JSON array). It
//! names no other bounded context.

pub mod agent_text;
pub mod ai_assist_runner;
pub mod bulk_parser;
pub mod bulk_prompt;
pub mod context_draft;
pub mod ports;
pub mod pull_request_prompt;
pub mod suggested_pull_request;
