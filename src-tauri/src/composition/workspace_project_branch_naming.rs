//! `ai_assist`'s `ProjectBranchNaming`, read from `workspace` Projects. The
//! only place `ai_assist`'s branch-naming port meets `workspace`: `ai_assist`
//! depends on the port, `workspace` owns the policy, and this adapter bridges
//! them so `ai_assist` never imports `workspace`.
//!
//! It names a proposed branch exactly as the `workspace_suggest_branch_name`
//! command does — through the project's `BranchNamingPolicy::compose_branch_name`
//! with the default prefix — so a drafted branch is valid the same way a
//! hand-typed one is. A blank slug or an unknown project is a naming failure,
//! and the use case drops that draft.

use std::sync::Arc;

use crate::ai_assist::domain::ports::{BranchNamingUnavailable, ProjectBranchNaming};
use crate::workspace::application::service::WorkspaceService;
use crate::workspace::domain::project_tag::ProjectTag;

pub struct WorkspaceProjectBranchNaming {
    workspace: Arc<WorkspaceService>,
}

impl WorkspaceProjectBranchNaming {
    pub fn new(workspace: Arc<WorkspaceService>) -> Self {
        Self { workspace }
    }
}

impl ProjectBranchNaming for WorkspaceProjectBranchNaming {
    fn name_branch(
        &self,
        project_tag: &str,
        proposed_name: &str,
    ) -> Result<String, BranchNamingUnavailable> {
        let tag = ProjectTag::parse(project_tag).map_err(|_| BranchNamingUnavailable)?;
        let project = self
            .workspace
            .find_project(&tag)
            .map_err(|_| BranchNamingUnavailable)?;
        let prefix = project
            .branch_naming()
            .default_prefix()
            .map(|prefix| prefix.as_str().to_string());
        project
            .branch_naming()
            .compose_branch_name(proposed_name, prefix.as_deref())
            .map(|branch| branch.as_str().to_string())
            .map_err(|_| BranchNamingUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::infra::in_memory::InMemoryGitRepositories;
    use crate::workspace::domain::ports::ProjectRepository;
    use crate::workspace::domain::test_support::{project_with, repository, PROJECT_TAG};
    use crate::workspace::infra::in_memory_agent_run_repository::InMemoryAgentRunRepository;
    use crate::workspace::infra::in_memory_project_repository::InMemoryProjectRepository;
    use crate::workspace::infra::recording_fakes::{
        RecordingCheckoutFiles, RecordingProjectEvents,
    };

    const UNKNOWN_PROJECT_TAG: &str = "unknown";
    const REPOSITORY_ID: &str = "api-id";
    const REPOSITORY_NAME: &str = "api";

    /// A project with the default branch naming policy: it offers the default
    /// prefixes but allows no prefix, so the default prefix is none and names
    /// are the bare slug.
    fn branch_naming() -> WorkspaceProjectBranchNaming {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let project = project_with(vec![repository(REPOSITORY_ID, REPOSITORY_NAME, "./api")]);
        projects.save(&project).unwrap();
        let git = Arc::new(InMemoryGitRepositories::new());
        let workspace = WorkspaceService::new(
            projects,
            git,
            Arc::new(RecordingCheckoutFiles::default()),
            Arc::new(RecordingProjectEvents::default()),
            Arc::new(InMemoryAgentRunRepository::default()),
        );
        WorkspaceProjectBranchNaming::new(Arc::new(workspace))
    }

    #[test]
    fn names_a_branch_from_a_proposed_slug_under_the_policy() {
        let branch = branch_naming().name_branch(PROJECT_TAG, "Add login page");

        assert_eq!(branch, Ok("add-login-page".to_string()));
    }

    #[test]
    fn a_name_with_no_letters_or_digits_cannot_be_named() {
        let branch = branch_naming().name_branch(PROJECT_TAG, "!!!");

        assert_eq!(branch, Err(BranchNamingUnavailable));
    }

    #[test]
    fn an_unknown_project_cannot_be_named() {
        let branch = branch_naming().name_branch(UNKNOWN_PROJECT_TAG, "Add login");

        assert_eq!(branch, Err(BranchNamingUnavailable));
    }
}
