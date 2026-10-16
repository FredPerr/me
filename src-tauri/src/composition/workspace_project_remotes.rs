//! `git_hosting`'s `ProjectRemotes`, read from `workspace` Projects and the
//! `origin` remote `git` finds in each repository's main checkout.

use std::sync::Arc;

use async_trait::async_trait;

use crate::git::domain::paths::RepositoryPath;
use crate::git::domain::ports::GitRepositories;
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::ports::ProjectRemotes;
use crate::git_hosting::domain::project_pull_requests::ProjectRepositoryRemote;
use crate::workspace::application::error::WorkspaceServiceError;
use crate::workspace::application::service::WorkspaceService;
use crate::workspace::domain::project_tag::ProjectTag;

#[derive(Clone)]
pub struct WorkspaceProjectRemotes {
    workspace: Arc<WorkspaceService>,
    git: Arc<dyn GitRepositories>,
}

impl WorkspaceProjectRemotes {
    pub fn new(workspace: Arc<WorkspaceService>, git: Arc<dyn GitRepositories>) -> Self {
        Self { workspace, git }
    }

    /// A repository that cannot be opened has no remote to list.
    fn read(&self, project_tag: &str) -> Result<Vec<ProjectRepositoryRemote>, GitHostingError> {
        let tag = ProjectTag::parse(project_tag).map_err(|_| GitHostingError::NotFound)?;
        let project = self.workspace.find_project(&tag).map_err(project_error)?;
        Ok(project
            .effective_repositories()
            .iter()
            .map(|repository| ProjectRepositoryRemote {
                repository_id: repository.id.as_str().to_string(),
                repository_name: repository.name.clone(),
                remote: self
                    .git
                    .open(&RepositoryPath::new(project.repository_path(repository)))
                    .ok()
                    .and_then(|opened| opened.remote_url()),
            })
            .collect())
    }
}

fn project_error(error: WorkspaceServiceError) -> GitHostingError {
    match error {
        WorkspaceServiceError::ProjectNotFound { .. } => GitHostingError::NotFound,
        other => {
            eprintln!("[git_hosting] project repositories not read: {other:?}");
            GitHostingError::Storage
        }
    }
}

#[async_trait]
impl ProjectRemotes for WorkspaceProjectRemotes {
    async fn repositories(
        &self,
        project_tag: &str,
    ) -> Result<Vec<ProjectRepositoryRemote>, GitHostingError> {
        let project_remotes = self.clone();
        let project_tag = project_tag.to_string();
        tauri::async_runtime::spawn_blocking(move || project_remotes.read(&project_tag))
            .await
            .map_err(|error| {
                eprintln!("[git_hosting] project repositories task failed: {error}");
                GitHostingError::Storage
            })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::domain::remote_url::RemoteUrl;
    use crate::git::infra::in_memory::InMemoryGitRepositories;
    use crate::git_hosting::domain::ports::ProjectRemotes;
    use crate::workspace::domain::ports::ProjectRepository;
    use crate::workspace::domain::test_support::{
        branch_name, project_with, repository, PROJECT_PATH, PROJECT_TAG,
    };
    use crate::workspace::infra::in_memory_project_repository::InMemoryProjectRepository;
    use crate::workspace::infra::recording_fakes::{
        RecordingCheckoutFiles, RecordingProjectEvents,
    };

    const API_ID: &str = "api-id";
    const API_NAME: &str = "api";
    const WEB_ID: &str = "web-id";
    const WEB_NAME: &str = "web";
    const DOCS_ID: &str = "docs-id";
    const DOCS_NAME: &str = "docs";
    const UNKNOWN_PROJECT_TAG: &str = "unknown";
    const BLANK_PROJECT_TAG: &str = "  ";
    const API_REMOTE: &str = "git@github.com:acme/api.git";
    const MAIN: &str = "main";

    fn path_of(repository_name: &str) -> RepositoryPath {
        RepositoryPath::new(format!("{PROJECT_PATH}/{repository_name}"))
    }

    /// api has a remote, web has none and docs is not a repository on disk.
    fn project_remotes() -> WorkspaceProjectRemotes {
        let projects = Arc::new(InMemoryProjectRepository::default());
        projects
            .save(&project_with(vec![
                repository(API_ID, API_NAME, "./api"),
                repository(WEB_ID, WEB_NAME, "./web"),
                repository(DOCS_ID, DOCS_NAME, "./docs"),
            ]))
            .unwrap();
        let git = Arc::new(InMemoryGitRepositories::new());
        for name in [API_NAME, WEB_NAME] {
            git.add_repository(&path_of(name), &branch_name(MAIN));
        }
        git.set_remote(&path_of(API_NAME), RemoteUrl::new(API_REMOTE));
        let workspace = WorkspaceService::new(
            projects,
            git.clone(),
            Arc::new(RecordingCheckoutFiles::default()),
            Arc::new(RecordingProjectEvents::default()),
        );
        WorkspaceProjectRemotes::new(Arc::new(workspace), git)
    }

    fn remote_of(name: &str, id: &str, remote: Option<&str>) -> ProjectRepositoryRemote {
        ProjectRepositoryRemote {
            repository_id: id.to_string(),
            repository_name: name.to_string(),
            remote: remote.map(RemoteUrl::new),
        }
    }

    #[tokio::test]
    async fn lists_each_repository_with_its_remote() {
        let repositories = project_remotes().repositories(PROJECT_TAG).await;

        assert_eq!(
            repositories,
            Ok(vec![
                remote_of(API_NAME, API_ID, Some(API_REMOTE)),
                remote_of(WEB_NAME, WEB_ID, None),
                remote_of(DOCS_NAME, DOCS_ID, None),
            ])
        );
    }

    #[tokio::test]
    async fn an_unknown_project_is_not_found() {
        for project_tag in [UNKNOWN_PROJECT_TAG, BLANK_PROJECT_TAG] {
            assert_eq!(
                project_remotes().repositories(project_tag).await,
                Err(GitHostingError::NotFound),
                "{project_tag}"
            );
        }
    }
}
