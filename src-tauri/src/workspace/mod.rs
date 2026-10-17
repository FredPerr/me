//! Workspace bounded context: Projects, their repositories and their contexts.
//! Every change to a Project goes through the `Project` aggregate.
//!
//! This module is the composition root: the only place that names the redb
//! repository, the file system adapter and the Tauri event adapter.
pub mod application;
pub mod commands;
#[allow(dead_code)] // the remaining use cases arrive in later tickets of #36
pub mod domain;
pub mod infra;

use std::sync::Arc;

use application::service::WorkspaceService;
use domain::ports::{AgentRunRepository, ProjectRepository};
use infra::file_system_checkout_files::FileSystemCheckoutFiles;
use infra::redb_agent_run_repository::RedbAgentRunRepository;
use infra::redb_project_repository::RedbProjectRepository;
use infra::tauri_project_events::TauriProjectEvents;

use crate::git::domain::ports::GitRepositories;

/// The workspace service, held in Tauri state.
pub struct Workspace {
    pub service: Arc<WorkspaceService>,
    /// The agent-run transcript store. Built here so the run-driving use case
    /// can take it; `run_agent` in #80 is its first consumer, so nothing reads
    /// it yet.
    #[allow(dead_code)] // used by run_agent in #80
    pub agent_runs: Arc<dyn AgentRunRepository>,
}

pub fn build(
    app: tauri::AppHandle,
    database: Arc<redb::Database>,
    git: Arc<dyn GitRepositories>,
) -> Result<Workspace, Box<dyn std::error::Error>> {
    let projects: Arc<dyn ProjectRepository> =
        Arc::new(RedbProjectRepository::new(Arc::clone(&database))?);
    let agent_runs: Arc<dyn AgentRunRepository> = Arc::new(RedbAgentRunRepository::new(database)?);
    let service = WorkspaceService::new(
        projects,
        git,
        Arc::new(FileSystemCheckoutFiles),
        Arc::new(TauriProjectEvents::new(app)),
    );
    Ok(Workspace {
        service: Arc::new(service),
        agent_runs,
    })
}
