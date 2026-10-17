//! `work_tracking`'s `ProjectTaskLinks`, read from `workspace` Projects: their
//! remote project links and the tasks their contexts were created from. Lets
//! the pending-tasks use case see the Project without `work_tracking` naming
//! `workspace`, mirroring `WorkspaceProjectRemotes` in git_hosting.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;

use crate::work_tracking::domain::error::{InputField, WorkTrackingError};
use crate::work_tracking::domain::identifiers::{ConnectionId, RemoteProjectId, TaskId};
use crate::work_tracking::domain::pending_tasks::{
    CreatedFromTask, ProjectLinks, ProjectTaskLink, ProjectTaskLinks,
};
use crate::workspace::application::error::WorkspaceServiceError;
use crate::workspace::application::service::WorkspaceService;
use crate::workspace::domain::context_origin::ContextOrigin;
use crate::workspace::domain::project::Project;
use crate::workspace::domain::project_tag::ProjectTag;

#[derive(Clone)]
pub struct WorkspaceProjectTaskLinks {
    workspace: Arc<WorkspaceService>,
}

impl WorkspaceProjectTaskLinks {
    pub fn new(workspace: Arc<WorkspaceService>) -> Self {
        Self { workspace }
    }

    fn read(&self, project_tag: &str) -> Result<ProjectLinks, WorkTrackingError> {
        let project = self.project(project_tag)?;
        Ok(ProjectLinks {
            links: links_of(&project),
            created_from: created_from_of(&project),
        })
    }

    fn project(&self, project_tag: &str) -> Result<Project, WorkTrackingError> {
        let tag = ProjectTag::parse(project_tag).map_err(|_| WorkTrackingError::NotFound)?;
        self.workspace.find_project(&tag).map_err(project_error)
    }
}

/// A link whose connection id or remote project id does not parse is dropped:
/// it could never be queried, so it cannot produce pending tasks.
fn links_of(project: &Project) -> Vec<ProjectTaskLink> {
    project
        .remote_project_links()
        .iter()
        .filter_map(|link| {
            Some(ProjectTaskLink {
                connection_id: ConnectionId::parse(&link.connection_id, InputField::ConnectionId)
                    .ok()?,
                remote_project_id: RemoteProjectId::parse(
                    &link.remote_project_id,
                    InputField::ProjectId,
                )
                .ok()?,
            })
        })
        .collect()
}

fn created_from_of(project: &Project) -> HashSet<CreatedFromTask> {
    project
        .contexts()
        .iter()
        .filter_map(|context| match &context.origin {
            ContextOrigin::FromTask(task) => Some(CreatedFromTask {
                connection_id: ConnectionId::parse(&task.connection_id, InputField::ConnectionId)
                    .ok()?,
                task_id: TaskId::parse(&task.task_id, InputField::ItemId).ok()?,
            }),
            _ => None,
        })
        .collect()
}

fn project_error(error: WorkspaceServiceError) -> WorkTrackingError {
    match error {
        WorkspaceServiceError::ProjectNotFound { .. } => WorkTrackingError::NotFound,
        other => {
            eprintln!("[work_tracking] project links not read: {other:?}");
            WorkTrackingError::StorageError
        }
    }
}

#[async_trait]
impl ProjectTaskLinks for WorkspaceProjectTaskLinks {
    async fn links_for(&self, project_tag: &str) -> Result<ProjectLinks, WorkTrackingError> {
        let project_tag = project_tag.to_string();
        let project_task_links = self.clone();
        // The Workspace reads from disk; keep it off the async runtime.
        tauri::async_runtime::spawn_blocking(move || project_task_links.read(&project_tag))
            .await
            .map_err(|error| {
                eprintln!("[work_tracking] project links task failed: {error}");
                WorkTrackingError::StorageError
            })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::infra::in_memory::InMemoryGitRepositories;
    use crate::workspace::domain::context::Context;
    use crate::workspace::domain::context_name::ContextName;
    use crate::workspace::domain::context_origin::TaskRef;
    use crate::workspace::domain::ports::ProjectRepository;
    use crate::workspace::domain::remote_project_link::RemoteProjectLink;
    use crate::workspace::domain::test_support::{project_with, PROJECT_TAG};
    use crate::workspace::infra::in_memory_agent_run_repository::InMemoryAgentRunRepository;
    use crate::workspace::infra::in_memory_project_repository::InMemoryProjectRepository;
    use crate::workspace::infra::recording_fakes::{
        RecordingCheckoutFiles, RecordingProjectEvents,
    };

    const CONNECTION_ID: &str = "teamwork:acme.teamwork.com";
    const OTHER_CONNECTION_ID: &str = "teamwork:other.teamwork.com";
    const REMOTE_PROJECT_ID: &str = "42";
    const OTHER_REMOTE_PROJECT_ID: &str = "43";
    const UNKNOWN_PROJECT_TAG: &str = "unknown";
    const TASK_ID: &str = "101";

    fn link(connection_id: &str, remote_project_id: &str) -> RemoteProjectLink {
        RemoteProjectLink {
            provider_kind: "teamwork".into(),
            connection_id: connection_id.into(),
            remote_project_id: remote_project_id.into(),
            remote_project_name: "Website".into(),
            remote_project_url: None,
            linked_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    fn context_from_task(name: &str, connection_id: &str, task_id: &str) -> Context {
        Context {
            origin: ContextOrigin::FromTask(TaskRef {
                connection_id: connection_id.into(),
                task_id: task_id.into(),
            }),
            ..Context::new(ContextName::parse(name).unwrap(), vec![]).unwrap()
        }
    }

    fn project_task_links(project: Project) -> WorkspaceProjectTaskLinks {
        let projects = Arc::new(InMemoryProjectRepository::default());
        projects.save(&project).unwrap();
        let git = Arc::new(InMemoryGitRepositories::new());
        let workspace = WorkspaceService::new(
            projects,
            git,
            Arc::new(RecordingCheckoutFiles::default()),
            Arc::new(RecordingProjectEvents::default()),
            Arc::new(InMemoryAgentRunRepository::default()),
        );
        WorkspaceProjectTaskLinks::new(Arc::new(workspace))
    }

    #[tokio::test]
    async fn maps_links_in_order_and_created_from_origins() {
        let project = project_with(vec![])
            .add_context(context_from_task("task-context", CONNECTION_ID, TASK_ID))
            .unwrap();
        let settings = settings_with_links(
            &project,
            vec![
                link(CONNECTION_ID, REMOTE_PROJECT_ID),
                link(OTHER_CONNECTION_ID, OTHER_REMOTE_PROJECT_ID),
            ],
        );
        let project = project.update_settings(settings).unwrap();

        let links = project_task_links(project)
            .links_for(PROJECT_TAG)
            .await
            .unwrap();

        assert_eq!(
            links
                .links
                .iter()
                .map(|link| (link.connection_id.as_str(), link.remote_project_id.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (CONNECTION_ID, REMOTE_PROJECT_ID),
                (OTHER_CONNECTION_ID, OTHER_REMOTE_PROJECT_ID),
            ]
        );
        assert_eq!(
            links.created_from,
            HashSet::from([CreatedFromTask {
                connection_id: ConnectionId::parse(CONNECTION_ID, InputField::ConnectionId)
                    .unwrap(),
                task_id: TaskId::parse(TASK_ID, InputField::ItemId).unwrap(),
            }])
        );
    }

    #[tokio::test]
    async fn a_project_without_links_or_task_contexts_has_neither() {
        let links = project_task_links(project_with(vec![]))
            .links_for(PROJECT_TAG)
            .await
            .unwrap();
        assert_eq!(links, ProjectLinks::default());
    }

    #[tokio::test]
    async fn an_unknown_project_is_not_found() {
        for project_tag in [UNKNOWN_PROJECT_TAG, "  "] {
            assert_eq!(
                project_task_links(project_with(vec![]))
                    .links_for(project_tag)
                    .await,
                Err(WorkTrackingError::NotFound),
                "{project_tag}"
            );
        }
    }

    /// Rebuilds the project settings, replacing the remote project links.
    fn settings_with_links(
        project: &Project,
        links: Vec<RemoteProjectLink>,
    ) -> crate::workspace::domain::project::ProjectSettings {
        crate::workspace::domain::project::ProjectSettings {
            name: project.name().to_string(),
            path: project.path().to_string(),
            icon: project.icon().map(str::to_string),
            repositories: project.repositories().to_vec(),
            symlinks: project.symlinks().to_vec(),
            remote_project_links: links,
            branch_naming: project.branch_naming().clone(),
        }
    }
}
