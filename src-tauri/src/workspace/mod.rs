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

use std::path::Path;
use std::sync::Arc;

use application::service::WorkspaceService;
use domain::ports::ProjectRepository;
use infra::file_system_checkout_files::FileSystemCheckoutFiles;
use infra::redb_project_repository::RedbProjectRepository;
use infra::tauri_project_events::TauriProjectEvents;

use crate::git::domain::ports::GitRepositories;

const DATABASE_FILE: &str = "me.redb";

/// The workspace service, held in Tauri state.
pub struct Workspace {
    pub service: Arc<WorkspaceService>,
}

pub fn build(
    app: tauri::AppHandle,
    app_data_folder: &Path,
    git: Arc<dyn GitRepositories>,
) -> Result<Workspace, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(app_data_folder)?;
    let projects: Arc<dyn ProjectRepository> = Arc::new(RedbProjectRepository::open(
        app_data_folder.join(DATABASE_FILE),
    )?);
    let service = WorkspaceService::new(
        projects,
        git,
        Arc::new(FileSystemCheckoutFiles),
        Arc::new(TauriProjectEvents::new(app)),
    );
    Ok(Workspace {
        service: Arc::new(service),
    })
}
