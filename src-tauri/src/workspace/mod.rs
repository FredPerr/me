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
use infra::legacy_project_import::{import_if_needed, UnreadableFile};
use infra::redb_project_repository::RedbProjectRepository;
use infra::tauri_project_events::TauriProjectEvents;

use crate::git::domain::ports::GitRepositories;

const DATABASE_FILE: &str = "me.redb";
const LEGACY_PROJECTS_FOLDER: &str = "projects";

/// Imports the old `projects/<tag>.json` files once, on first launch, renaming
/// each imported file to `.json.imported`.
const RUN_LEGACY_IMPORT_AT_STARTUP: bool = true;

/// The workspace service and what the one-time import couldn't read, held in Tauri state.
pub struct Workspace {
    pub service: Arc<WorkspaceService>,
    pub unreadable_files: Vec<UnreadableFile>,
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
    let unreadable_files = if RUN_LEGACY_IMPORT_AT_STARTUP {
        import_legacy_projects(
            &app_data_folder.join(LEGACY_PROJECTS_FOLDER),
            projects.as_ref(),
        )
    } else {
        vec![]
    };
    let service = WorkspaceService::new(
        projects,
        git,
        Arc::new(FileSystemCheckoutFiles),
        Arc::new(TauriProjectEvents::new(app)),
    );
    Ok(Workspace {
        service: Arc::new(service),
        unreadable_files,
    })
}

/// TEMPORARY(legacy import): remove with `infra::legacy_project_import`.
fn import_legacy_projects(
    projects_folder: &Path,
    projects: &dyn ProjectRepository,
) -> Vec<UnreadableFile> {
    match import_if_needed(projects_folder, projects) {
        Ok(report) => {
            if !report.imported.is_empty() || !report.unreadable.is_empty() {
                eprintln!(
                    "[workspace] imported {} project file(s); left {} unreadable; skipped {} context(s)",
                    report.imported.len(),
                    report.unreadable.len(),
                    report.skipped_contexts.len()
                );
            }
            for skipped in &report.skipped_contexts {
                eprintln!(
                    "[workspace] context '{}' of '{}' not imported: {}",
                    skipped.context_name, skipped.project, skipped.reason
                );
            }
            report.unreadable
        }
        Err(error) => {
            eprintln!("[workspace] project files not imported: {error}");
            vec![]
        }
    }
}
