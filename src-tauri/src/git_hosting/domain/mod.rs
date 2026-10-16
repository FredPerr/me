//! Host-agnostic model of the Git Hosting context.

pub mod capped;
pub mod error;
pub mod host_kind;
pub mod host_secret;
pub mod hosted_account;
pub mod hosted_pull_request;
pub mod hosted_repository;
pub mod new_pull_request;
pub mod ports;
pub mod project_pull_requests;
pub mod sign_in;
