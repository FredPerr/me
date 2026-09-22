//! Git provider authentication and API access.
//!
//! Each provider (GitHub today, GitLab/Bitbucket later) lives in its own
//! submodule. Access tokens are never persisted to the plaintext app store;
//! they are kept in the OS keychain via [`token_store`], keyed by provider id.

pub mod git_auth;
pub mod github;
pub mod token_store;

/// Stable identifiers for the supported git providers. Kept as plain strings
/// on the wire so the frontend port can map them without a shared enum crate.
pub const PROVIDER_GITHUB: &str = "github";
