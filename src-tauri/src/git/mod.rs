//! Git bounded context: branches, worktrees and remotes of one repository at a
//! time. It knows nothing about Projects or Contexts.
//!
//! This module is the composition root: the only place that names the git
//! adapter and the credentials it uses.
pub mod commands;
pub mod domain;
pub mod infra;

use std::sync::Arc;

use domain::ports::{GitRepositories, RemoteCredentials};
use infra::git2_cli::Git2CliRepositories;

/// Shared access to repositories, held in Tauri state.
#[derive(Clone)]
pub struct Git(Arc<dyn GitRepositories>);

impl Git {
    pub fn repositories(&self) -> Arc<dyn GitRepositories> {
        self.0.clone()
    }
}

pub fn build(credentials: Arc<dyn RemoteCredentials>) -> Git {
    Git(Arc::new(Git2CliRepositories::new(credentials)))
}

/// Runs blocking git work off the async runtime, reporting a failed task as `E`.
pub async fn run_blocking<T, E, F>(work: F) -> Result<T, E>
where
    T: Send + 'static,
    E: From<domain::error::GitError> + Send + 'static,
    F: FnOnce() -> Result<T, E> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| E::from(domain::error::GitError::io("run the git task", error)))?
}
