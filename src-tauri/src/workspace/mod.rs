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

use application::agent_runs::AgentRunService;
use application::service::WorkspaceService;
use domain::agent_runner::AgentRunner;
use domain::ports::{AgentRunRepository, DefaultAgentTool, ProjectEvents, ProjectRepository};
use infra::file_system_checkout_files::FileSystemCheckoutFiles;
use infra::file_system_permission_floor::FileSystemPermissionFloor;
use infra::redb_agent_run_repository::RedbAgentRunRepository;
use infra::redb_project_repository::RedbProjectRepository;
use infra::system_clock::SystemClock;
use infra::tauri_agent_run_events::TauriAgentRunEvents;
use infra::tauri_project_events::TauriProjectEvents;

use crate::git::domain::ports::GitRepositories;

/// The workspace services, held in Tauri state.
pub struct Workspace {
    pub service: Arc<WorkspaceService>,
    /// Drives agent runs for contexts: status, transcript, conversation id and
    /// the live `agent-run` event. Shares the project repository with `service`.
    pub agent_run_service: Arc<AgentRunService>,
}

/// Builds the workspace services. The agent `runner` and the `default_tool`
/// (the Default AI tool a fresh context's first run uses, #90) are injected
/// because they are composition adapters over `ai_session` and `settings`,
/// built in the composition root; the event emitter, clock and permission
/// floor are workspace infra built here.
pub fn build(
    app: tauri::AppHandle,
    database: Arc<redb::Database>,
    git: Arc<dyn GitRepositories>,
    runner: Arc<dyn AgentRunner>,
    default_tool: Arc<dyn DefaultAgentTool>,
) -> Result<Workspace, Box<dyn std::error::Error>> {
    let projects: Arc<dyn ProjectRepository> =
        Arc::new(RedbProjectRepository::new(Arc::clone(&database))?);
    let agent_runs: Arc<dyn AgentRunRepository> = Arc::new(RedbAgentRunRepository::new(database)?);
    let project_events: Arc<dyn ProjectEvents> = Arc::new(TauriProjectEvents::new(app.clone()));
    let service = WorkspaceService::new(
        Arc::clone(&projects),
        git,
        Arc::new(FileSystemCheckoutFiles),
        Arc::clone(&project_events),
        Arc::clone(&agent_runs),
    );
    let agent_run_service = AgentRunService::new(
        projects,
        agent_runs,
        runner,
        Arc::new(TauriAgentRunEvents::new(app)),
        project_events,
        Arc::new(SystemClock),
        Arc::new(FileSystemPermissionFloor),
        default_tool,
    );
    Ok(Workspace {
        service: Arc::new(service),
        agent_run_service: Arc::new(agent_run_service),
    })
}
