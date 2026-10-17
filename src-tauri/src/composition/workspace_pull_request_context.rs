//! `ai_assist`'s `PullRequestContext`, resolved from `workspace` Projects. The
//! only place `ai_assist`'s pull-request-target port meets `workspace`:
//! `ai_assist` depends on the port, `workspace` owns the Projects, and this
//! adapter bridges them so `ai_assist` never imports `workspace`.
//!
//! It resolves the same four values the create-pull-request modal computes: the
//! context and repository names, the worktree path (`Project::worktree_path`),
//! and the base branch (`Project::base_branch_for` — the base context's branch,
//! else the default context's). A missing base branch is `NoBaseBranch`; an
//! unknown project, context or repository is `NotFound`.

use std::sync::Arc;

use crate::ai_assist::domain::ports::{
    PullRequestContext, PullRequestContextError, PullRequestTarget,
};
use crate::workspace::application::service::WorkspaceService;
use crate::workspace::domain::identifiers::{ContextId, RepositoryId};
use crate::workspace::domain::project_tag::ProjectTag;

pub struct WorkspacePullRequestContext {
    workspace: Arc<WorkspaceService>,
}

impl WorkspacePullRequestContext {
    pub fn new(workspace: Arc<WorkspaceService>) -> Self {
        Self { workspace }
    }
}

impl PullRequestContext for WorkspacePullRequestContext {
    fn resolve(
        &self,
        project_tag: &str,
        context_id: &str,
        repository_id: &str,
    ) -> Result<PullRequestTarget, PullRequestContextError> {
        let tag = ProjectTag::parse(project_tag).map_err(not_found)?;
        let context_id = ContextId::parse(context_id).map_err(not_found)?;
        let repository_id = RepositoryId::parse(repository_id).map_err(not_found)?;

        // load_project mirrors the frontend's repaired default context, so the
        // default-branch fallback sees a branch per repository.
        let project = self.workspace.load_project(&tag).map_err(not_found)?;
        let context =
            project
                .find_context(&context_id)
                .ok_or_else(|| PullRequestContextError::NotFound {
                    reason: format!("context '{context_id}' was not found"),
                })?;
        let repository = project
            .find_effective_repository(&repository_id)
            .ok_or_else(|| PullRequestContextError::NotFound {
                reason: format!("repository '{repository_id}' was not found"),
            })?;
        let base_branch = project
            .base_branch_for(context, &repository_id)
            .ok_or(PullRequestContextError::NoBaseBranch)?;

        Ok(PullRequestTarget {
            context_name: context.name.to_string(),
            repository_name: repository.name.clone(),
            worktree_path: project.worktree_path(context, &repository),
            base_branch: base_branch.to_string(),
        })
    }
}

fn not_found(error: impl std::fmt::Display) -> PullRequestContextError {
    PullRequestContextError::NotFound {
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::infra::in_memory::InMemoryGitRepositories;
    use crate::workspace::domain::ports::ProjectRepository;
    use crate::workspace::domain::test_support::{
        branch_on, context_named, project_with, repository, PROJECT_TAG,
    };
    use crate::workspace::infra::in_memory_agent_run_repository::InMemoryAgentRunRepository;
    use crate::workspace::infra::in_memory_project_repository::InMemoryProjectRepository;
    use crate::workspace::infra::recording_fakes::{
        RecordingCheckoutFiles, RecordingProjectEvents,
    };

    const REPOSITORY_ID: &str = "api-id";
    const REPOSITORY_NAME: &str = "api";
    const MAIN_BRANCH: &str = "main";
    const UNKNOWN_ID: &str = "nope";

    /// A workspace with one project: a default context branched on `main` for
    /// the single repository, and a feature context branched on `feature/x`.
    fn workspace_with_feature_context() -> (Arc<WorkspaceService>, String) {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let repositories = vec![repository(REPOSITORY_ID, REPOSITORY_NAME, "./api")];
        let default_context =
            crate::workspace::domain::context::Context::new_default(vec![branch_on(
                REPOSITORY_ID,
                MAIN_BRANCH,
            )]);
        let mut feature = context_named("Add login");
        feature.branches = vec![branch_on(REPOSITORY_ID, "feature/x")];
        let feature_id = feature.id.to_string();
        let project = project_with(repositories)
            .add_context(default_context)
            .unwrap()
            .add_context(feature)
            .unwrap();
        projects.save(&project).unwrap();

        let git = Arc::new(InMemoryGitRepositories::new());
        let workspace = WorkspaceService::new(
            projects,
            git,
            Arc::new(RecordingCheckoutFiles::default()),
            Arc::new(RecordingProjectEvents::default()),
            Arc::new(InMemoryAgentRunRepository::default()),
        );
        (Arc::new(workspace), feature_id)
    }

    #[test]
    fn resolves_the_names_worktree_and_base_branch() {
        let (workspace, feature_id) = workspace_with_feature_context();
        let adapter = WorkspacePullRequestContext::new(workspace);

        let target = adapter
            .resolve(PROJECT_TAG, &feature_id, REPOSITORY_ID)
            .unwrap();

        assert_eq!(target.context_name, "Add login");
        assert_eq!(target.repository_name, REPOSITORY_NAME);
        assert_eq!(target.base_branch, MAIN_BRANCH);
        assert!(target
            .worktree_path
            .ends_with(&format!("add-login/{REPOSITORY_NAME}")));
    }

    #[test]
    fn an_unknown_context_is_not_found() {
        let (workspace, _) = workspace_with_feature_context();
        let adapter = WorkspacePullRequestContext::new(workspace);

        let result = adapter.resolve(PROJECT_TAG, UNKNOWN_ID, REPOSITORY_ID);

        assert!(matches!(
            result,
            Err(PullRequestContextError::NotFound { .. })
        ));
    }

    #[test]
    fn an_unknown_repository_is_not_found() {
        let (workspace, feature_id) = workspace_with_feature_context();
        let adapter = WorkspacePullRequestContext::new(workspace);

        let result = adapter.resolve(PROJECT_TAG, &feature_id, UNKNOWN_ID);

        assert!(matches!(
            result,
            Err(PullRequestContextError::NotFound { .. })
        ));
    }
}
