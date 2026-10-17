//! AI-assist application layer: the use cases that orchestrate the runner and
//! the ports into a finished result. This ticket (#88) adds `suggest_contexts`;
//! #89 adds `suggest_pull_request` alongside it.

pub mod error;
pub mod suggest_contexts;
