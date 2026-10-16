//! Git bounded context: branches, worktrees and remotes of one repository at a
//! time. It knows nothing about Projects or Contexts.
//!
//! This module is the composition root: the only place that names the git
//! adapter and the credentials it uses.
pub mod commands;
pub mod domain;
pub mod infra;

use std::sync::Arc;

use domain::ports::GitRepositories;
use infra::git2_cli::Git2CliRepositories;
use infra::provider_token_credentials::ProviderTokenCredentials;

/// Shared access to repositories, held in Tauri state.
#[derive(Clone)]
pub struct Git(Arc<dyn GitRepositories>);

impl Git {
    pub fn repositories(&self) -> Arc<dyn GitRepositories> {
        self.0.clone()
    }
}

pub fn build() -> Git {
    Git(Arc::new(Git2CliRepositories::new(Arc::new(
        ProviderTokenCredentials,
    ))))
}

/// Runs blocking git work off the async runtime.
pub async fn run_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| format!("Git task failed: {error}"))?
}
