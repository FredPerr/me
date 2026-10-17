//! `git_hosting`'s `ProjectRemotes`, read from `workspace` Projects (their
//! repositories and the branch each context uses in them) and the `origin`
//! remote `git` finds in each repository's main checkout.

use std::sync::Arc;

use async_trait::async_trait;

use crate::git::domain::paths::RepositoryPath;
use crate::git::domain::ports::GitRepositories;
use crate::git::domain::remote_url::RemoteUrl;
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::ports::ProjectRemotes;
use crate::git_hosting::domain::project_pull_requests::{
    ProjectRepositoryRemote, RepositoryBranchCheckout,
};
use crate::workspace::application::error::WorkspaceServiceError;
use crate::workspace::application::service::WorkspaceService;
use crate::workspace::domain::project::Project;
use crate::workspace::domain::project_tag::ProjectTag;
use crate::workspace::domain::repository::Repository;

#[derive(Clone)]
pub struct WorkspaceProjectRemotes {
    workspace: Arc<WorkspaceService>,
    git: Arc<dyn GitRepositories>,
}

impl WorkspaceProjectRemotes {
    pub fn new(workspace: Arc<WorkspaceService>, git: Arc<dyn GitRepositories>) -> Self {
        Self { workspace, git }
    }

    fn read(&self, project_tag: &str) -> Result<Vec<ProjectRepositoryRemote>, GitHostingError> {
        let project = self.project(project_tag)?;
        Ok(project
            .effective_repositories()
            .iter()
            .map(|repository| ProjectRepositoryRemote {
                repository_id: repository.id.as_str().to_string(),
                repository_name: repository.name.clone(),
                remote: self.remote_of(&project, repository),
                context_branches: project
                    .contexts()
                    .iter()
                    .filter_map(|context| context.branch_for(&repository.id))
                    .map(|context_branch| context_branch.branch.as_str().to_string())
                    .collect(),
            })
            .collect())
    }

    fn read_branch_checkout(
        &self,
        project_tag: &str,
        repository_id: &str,
        branch: &str,
    ) -> Result<RepositoryBranchCheckout, GitHostingError> {
        let project = self.project(project_tag)?;
        let repository = project
            .effective_repositories()
            .iter()
            .find(|repository| repository.id.as_str() == repository_id)
            .cloned()
            .ok_or(GitHostingError::NotFound)?;
        let worktree_path = project
            .contexts()
            .iter()
            .find(|context| {
                context
                    .branch_for(&repository.id)
                    .is_some_and(|context_branch| context_branch.branch.as_str() == branch)
            })
            .map(|context| project.worktree_path(context, &repository))
            .ok_or(GitHostingError::NotFound)?;
        Ok(RepositoryBranchCheckout {
            remote: self.remote_of(&project, &repository),
            worktree_path,
        })
    }

    fn project(&self, project_tag: &str) -> Result<Project, GitHostingError> {
        let tag = ProjectTag::parse(project_tag).map_err(|_| GitHostingError::NotFound)?;
        self.workspace.find_project(&tag).map_err(project_error)
    }

    /// A repository that cannot be opened has no remote.
    fn remote_of(&self, project: &Project, repository: &Repository) -> Option<RemoteUrl> {
        self.git
            .open(&RepositoryPath::new(project.repository_path(repository)))
            .ok()
            .and_then(|opened| opened.remote_url())
    }

    /// Runs `read` off the async runtime: the Workspace and git read from disk.
    async fn read_blocking<T: Send + 'static>(
        &self,
        read: impl FnOnce(Self) -> Result<T, GitHostingError> + Send + 'static,
    ) -> Result<T, GitHostingError> {
        let project_remotes = self.clone();
        tauri::async_runtime::spawn_blocking(move || read(project_remotes))
            .await
            .map_err(|error| {
                eprintln!("[git_hosting] project repositories task failed: {error}");
                GitHostingError::Storage
            })?
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
        let project_tag = project_tag.to_string();
        self.read_blocking(move |project_remotes| project_remotes.read(&project_tag))
            .await
    }

    async fn branch_checkout(
        &self,
        project_tag: &str,
        repository_id: &str,
        branch: &str,
    ) -> Result<RepositoryBranchCheckout, GitHostingError> {
        let (project_tag, repository_id, branch) = (
            project_tag.to_string(),
            repository_id.to_string(),
            branch.to_string(),
        );
        self.read_blocking(move |project_remotes| {
            project_remotes.read_branch_checkout(&project_tag, &repository_id, &branch)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::domain::remote_url::RemoteUrl;
    use crate::git::infra::in_memory::InMemoryGitRepositories;
    use crate::git_hosting::domain::ports::ProjectRemotes;
    use crate::workspace::domain::context::Context;
    use crate::workspace::domain::context_branch::ContextBranch;
    use crate::workspace::domain::context_name::ContextName;
    use crate::workspace::domain::ports::ProjectRepository;
    use crate::workspace::domain::test_support::{
        branch_name, branch_on, project_with, repository, PROJECT_PATH, PROJECT_TAG,
    };
    use crate::workspace::infra::in_memory_agent_run_repository::InMemoryAgentRunRepository;
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
    const LOGIN_CONTEXT: &str = "login";
    const SEARCH_CONTEXT: &str = "search";
    const LOGIN_BRANCH: &str = "feature/login";
    const SEARCH_BRANCH: &str = "feature/search";

    fn context_on(name: &str, branches: Vec<ContextBranch>) -> Context {
        Context::new(ContextName::parse(name).unwrap(), branches).unwrap()
    }

    fn path_of(repository_name: &str) -> RepositoryPath {
        RepositoryPath::new(format!("{PROJECT_PATH}/{repository_name}"))
    }

    /// api has a remote, web has none and docs is not a repository on disk.
    /// The login context uses api and web, the search context only api.
    fn project_remotes() -> WorkspaceProjectRemotes {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let project = project_with(vec![
            repository(API_ID, API_NAME, "./api"),
            repository(WEB_ID, WEB_NAME, "./web"),
            repository(DOCS_ID, DOCS_NAME, "./docs"),
        ])
        .add_context(context_on(
            LOGIN_CONTEXT,
            vec![
                branch_on(API_ID, LOGIN_BRANCH),
                branch_on(WEB_ID, LOGIN_BRANCH),
            ],
        ))
        .unwrap()
        .add_context(context_on(
            SEARCH_CONTEXT,
            vec![branch_on(API_ID, SEARCH_BRANCH)],
        ))
        .unwrap();
        projects.save(&project).unwrap();
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
            Arc::new(InMemoryAgentRunRepository::default()),
        );
        WorkspaceProjectRemotes::new(Arc::new(workspace), git)
    }

    fn remote_of(
        name: &str,
        id: &str,
        remote: Option<&str>,
        context_branches: &[&str],
    ) -> ProjectRepositoryRemote {
        ProjectRepositoryRemote {
            repository_id: id.to_string(),
            repository_name: name.to_string(),
            remote: remote.map(RemoteUrl::new),
            context_branches: context_branches
                .iter()
                .map(|branch| branch.to_string())
                .collect(),
        }
    }

    #[tokio::test]
    async fn lists_each_repository_with_its_remote_and_context_branches() {
        let repositories = project_remotes().repositories(PROJECT_TAG).await;

        assert_eq!(
            repositories,
            Ok(vec![
                remote_of(
                    API_NAME,
                    API_ID,
                    Some(API_REMOTE),
                    &[LOGIN_BRANCH, SEARCH_BRANCH]
                ),
                remote_of(WEB_NAME, WEB_ID, None, &[LOGIN_BRANCH]),
                remote_of(DOCS_NAME, DOCS_ID, None, &[]),
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

    mod branch_checkout {
        use super::*;

        const UNKNOWN_REPOSITORY_ID: &str = "unknown-id";
        const UNUSED_BRANCH: &str = "feature/unused";

        #[tokio::test]
        async fn finds_the_remote_and_worktree_of_a_context_branch() {
            let checkout = project_remotes()
                .branch_checkout(PROJECT_TAG, API_ID, SEARCH_BRANCH)
                .await;

            assert_eq!(
                checkout,
                Ok(RepositoryBranchCheckout {
                    remote: Some(RemoteUrl::new(API_REMOTE)),
                    worktree_path: format!("{PROJECT_PATH}/.worktrees/{SEARCH_CONTEXT}/{API_NAME}"),
                })
            );
        }

        #[tokio::test]
        async fn a_repository_without_remote_has_none() {
            let checkout = project_remotes()
                .branch_checkout(PROJECT_TAG, WEB_ID, LOGIN_BRANCH)
                .await;

            assert_eq!(checkout.map(|checkout| checkout.remote), Ok(None));
        }

        #[tokio::test]
        async fn an_unknown_project_repository_or_branch_is_not_found() {
            for (project_tag, repository_id, branch) in [
                (UNKNOWN_PROJECT_TAG, API_ID, LOGIN_BRANCH),
                (PROJECT_TAG, UNKNOWN_REPOSITORY_ID, LOGIN_BRANCH),
                (PROJECT_TAG, API_ID, UNUSED_BRANCH),
                (PROJECT_TAG, DOCS_ID, LOGIN_BRANCH),
            ] {
                assert_eq!(
                    project_remotes()
                        .branch_checkout(project_tag, repository_id, branch)
                        .await,
                    Err(GitHostingError::NotFound),
                    "{project_tag} {repository_id} {branch}"
                );
            }
        }
    }
}
