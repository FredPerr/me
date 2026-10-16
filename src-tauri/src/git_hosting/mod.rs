//! Git Hosting bounded context: Git hosts such as GitHub and Bitbucket, the
//! repositories they host and the accounts signed in to them.
//!
//! This module is the composition root: the only place that names the
//! keychain adapter and the credentials handed to the `git` module.

// Used by later tickets of #55.
#[allow(dead_code)]
pub mod domain;
pub mod infra;

use std::sync::Arc;

use crate::git::domain::ports::RemoteCredentials;
use infra::host_token_credentials::HostTokenCredentials;
use infra::keychain_host_tokens::KeychainHostTokens;

/// Credentials `git` uses to push and pull over HTTPS with a signed-in Git host.
pub fn remote_credentials() -> Arc<dyn RemoteCredentials> {
    Arc::new(HostTokenCredentials::new(Arc::new(KeychainHostTokens)))
}
