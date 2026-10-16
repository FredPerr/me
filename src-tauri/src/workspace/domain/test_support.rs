//! Builders shared by the domain tests.

use crate::git::domain::branch_name::BranchName;

use super::branch_naming::BranchNamingPolicy;
use super::context::Context;
use super::context_branch::ContextBranch;
use super::context_name::ContextName;
use super::identifiers::RepositoryId;
use super::project::{Project, ProjectSettings};
use super::project_tag::ProjectTag;
use super::repository::Repository;

pub const PROJECT_NAME: &str = "My Project";
pub const PROJECT_TAG: &str = "my-project";
pub const PROJECT_PATH: &str = "/Users/me/projects/my-project";

pub fn repository_id(raw: &str) -> RepositoryId {
    RepositoryId::parse(raw).unwrap()
}

pub fn branch_name(raw: &str) -> BranchName {
    BranchName::parse(raw).unwrap()
}

pub fn branch_on(repository: &str, branch: &str) -> ContextBranch {
    ContextBranch::new(repository_id(repository), branch_name(branch))
}

pub fn repository(id: &str, name: &str, relative_path: &str) -> Repository {
    Repository::new(repository_id(id), name, relative_path)
}

pub fn context_named(name: &str) -> Context {
    Context::new(ContextName::parse(name).unwrap(), vec![]).unwrap()
}

pub fn settings(repositories: Vec<Repository>) -> ProjectSettings {
    ProjectSettings {
        name: PROJECT_NAME.to_string(),
        path: PROJECT_PATH.to_string(),
        icon: None,
        repositories,
        symlinks: vec![],
        remote_project_links: vec![],
        branch_naming: BranchNamingPolicy::default(),
    }
}

pub fn project_with(repositories: Vec<Repository>) -> Project {
    Project::new(
        ProjectTag::parse(PROJECT_TAG).unwrap(),
        settings(repositories),
    )
}
