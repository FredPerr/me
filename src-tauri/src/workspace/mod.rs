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
use application::workbench_service::WorkbenchService;
use domain::agent_runner::AgentRunner;
use domain::ports::{
    AgentRunRepository, DefaultAgentTool, ProjectEvents, ProjectRepository, WorkbenchEvents,
    WorkbenchRepository,
};
use infra::file_system_checkout_files::FileSystemCheckoutFiles;
use infra::file_system_permission_floor::FileSystemPermissionFloor;
use infra::redb_agent_run_repository::RedbAgentRunRepository;
use infra::redb_project_repository::RedbProjectRepository;
use infra::redb_workbench_repository::RedbWorkbenchRepository;
use infra::system_clock::SystemClock;
use infra::tauri_agent_run_events::TauriAgentRunEvents;
use infra::tauri_project_events::TauriProjectEvents;
use infra::tauri_workbench_events::TauriWorkbenchEvents;

use crate::git::domain::ports::GitRepositories;

/// The workspace services, held in Tauri state.
pub struct Workspace {
    pub service: Arc<WorkspaceService>,
    /// Drives agent runs for contexts: status, transcript, conversation id and
    /// the live `agent-run` event. Shares the project repository with `service`.
    pub agent_run_service: Arc<AgentRunService>,
    /// Drives the Workbench (the tab-bar arrangement): read the reconciled
    /// arrangement, change a project's placement, reorder a group, set the
    /// last-open project. Shares the project repository with `service` to
    /// reconcile against the live project list.
    pub workbench_service: Arc<WorkbenchService>,
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
    let agent_runs: Arc<dyn AgentRunRepository> =
        Arc::new(RedbAgentRunRepository::new(Arc::clone(&database))?);
    let workbench_repository: Arc<dyn WorkbenchRepository> =
        Arc::new(RedbWorkbenchRepository::new(database)?);
    let project_events: Arc<dyn ProjectEvents> = Arc::new(TauriProjectEvents::new(app.clone()));
    let workbench_events: Arc<dyn WorkbenchEvents> =
        Arc::new(TauriWorkbenchEvents::new(app.clone()));
    let service = WorkspaceService::new(
        Arc::clone(&projects),
        git,
        Arc::new(FileSystemCheckoutFiles),
        Arc::clone(&project_events),
        Arc::clone(&agent_runs),
    );
    let workbench_service = WorkbenchService::new(
        workbench_repository,
        Arc::clone(&projects),
        workbench_events,
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
        workbench_service: Arc::new(workbench_service),
    })
}
