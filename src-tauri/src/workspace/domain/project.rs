use std::borrow::Cow;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::git::domain::branch_name::BranchName;

use super::branch_naming::BranchNamingPolicy;
use super::context::Context;
use super::context_branch::{ContextBranch, PullRequestDraft, PullRequestLink};
use super::context_status::ContextStatus;
use super::default_branch::choose_default_branch;
use super::error::WorkspaceError;
use super::identifiers::{ContextId, RepositoryId};
use super::project_tag::ProjectTag;
use super::remote_project_link::RemoteProjectLink;
use super::repository::Repository;

/// A named, local codebase of one or more repositories worked on together,
/// with its contexts. The root of the workspace: every change goes through one
/// of its methods, which checks the rules and returns the updated Project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    tag: ProjectTag,
    name: String,
    path: String,
    icon: Option<String>,
    repositories: Vec<Repository>,
    contexts: Vec<Context>,
    symlinks: Vec<String>,
    #[serde(default)]
    remote_project_links: Vec<RemoteProjectLink>,
    #[serde(default)]
    branch_naming: BranchNamingPolicy,
}

/// Everything about a Project the settings form edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSettings {
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
    pub repositories: Vec<Repository>,
    pub symlinks: Vec<String>,
    pub remote_project_links: Vec<RemoteProjectLink>,
    pub branch_naming: BranchNamingPolicy,
}

impl ProjectSettings {
    /// Refuses a blank name or folder and two repositories with one id; drops
    /// blank symlinks and turns a blank icon or post-checkout command into none.
    fn checked(self) -> Result<Self, WorkspaceError> {
        if self.name.trim().is_empty() {
            return Err(WorkspaceError::EmptyProjectName);
        }
        if self.path.trim().is_empty() {
            return Err(WorkspaceError::EmptyProjectPath);
        }
        for (index, repository) in self.repositories.iter().enumerate() {
            if self.repositories[..index]
                .iter()
                .any(|earlier| earlier.id == repository.id)
            {
                return Err(WorkspaceError::DuplicateRepository {
                    id: repository.id.clone(),
                });
            }
        }
        Ok(Self {
            icon: self.icon.filter(|icon| !icon.trim().is_empty()),
            repositories: self
                .repositories
                .into_iter()
                .map(|repository| Repository {
                    post_checkout_command: repository
                        .post_checkout_command
                        .filter(|command| !command.trim().is_empty()),
                    ..repository
                })
                .collect(),
            symlinks: self
                .symlinks
                .into_iter()
                .filter(|symlink| !symlink.trim().is_empty())
                .collect(),
            ..self
        })
    }
}

impl Project {
    /// A project without contexts; `repair_default_context` gives it its default context.
    pub fn new(tag: ProjectTag, settings: ProjectSettings) -> Self {
        Self {
            tag,
            name: settings.name,
            path: settings.path,
            icon: settings.icon,
            repositories: settings.repositories,
            contexts: vec![],
            symlinks: settings.symlinks,
            remote_project_links: settings.remote_project_links,
            branch_naming: settings.branch_naming,
        }
    }

    pub fn tag(&self) -> &ProjectTag {
        &self.tag
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }

    /// The configured repositories, possibly none; see `effective_repositories`.
    pub fn repositories(&self) -> &[Repository] {
        &self.repositories
    }

    pub fn contexts(&self) -> &[Context] {
        &self.contexts
    }

    pub fn symlinks(&self) -> &[String] {
        &self.symlinks
    }

    pub fn remote_project_links(&self) -> &[RemoteProjectLink] {
        &self.remote_project_links
    }

    pub fn branch_naming(&self) -> &BranchNamingPolicy {
        &self.branch_naming
    }

    /// A new project with checked settings, still without contexts.
    pub fn create(tag: ProjectTag, settings: ProjectSettings) -> Result<Self, WorkspaceError> {
        Ok(Self::new(tag, settings.checked()?))
    }

    /// Replaces the settings after checking them, keeping the tag and the contexts.
    pub fn update_settings(self, settings: ProjectSettings) -> Result<Self, WorkspaceError> {
        Ok(Self {
            contexts: self.contexts,
            ..Self::new(self.tag, settings.checked()?)
        })
    }

    /// The configured repositories or, when there are none, the single
    /// repository at the project root, whose id is the project tag.
    pub fn effective_repositories(&self) -> Cow<'_, [Repository]> {
        if self.repositories.is_empty() {
            Cow::Owned(vec![self.root_repository()])
        } else {
            Cow::Borrowed(&self.repositories)
        }
    }

    pub fn find_effective_repository(&self, id: &RepositoryId) -> Option<Cow<'_, Repository>> {
        if self.repositories.is_empty() {
            let root = self.root_repository();
            return (&root.id == id).then_some(Cow::Owned(root));
        }
        self.repositories
            .iter()
            .find(|repository| &repository.id == id)
            .map(Cow::Borrowed)
    }

    fn root_repository(&self) -> Repository {
        Repository::new(self.tag.root_repository_id(), self.name.clone(), ".")
    }

    pub fn default_context(&self) -> Option<&Context> {
        self.contexts.iter().find(|context| context.is_default)
    }

    pub fn find_context(&self, id: &ContextId) -> Option<&Context> {
        self.contexts.iter().find(|context| &context.id == id)
    }

    /// The branch a context's changes in `repository` are opened against: the
    /// base context's branch there, else the default context's. `None` when
    /// neither has a branch for the repository. This is the single source of
    /// the base-branch fallback the create-pull-request flow uses.
    pub fn base_branch_for(
        &self,
        context: &Context,
        repository: &RepositoryId,
    ) -> Option<&BranchName> {
        let from_base_context = context
            .base
            .as_ref()
            .and_then(|base| self.find_context(base))
            .and_then(|base| base.branch_for(repository));
        from_base_context
            .or_else(|| self.default_context()?.branch_for(repository))
            .map(|branch| &branch.branch)
    }

    /// Adds a context after checking its name, folder, base context and
    /// branches. A default context is only accepted while there is none, and
    /// is made idle and static.
    pub fn add_context(self, context: Context) -> Result<Self, WorkspaceError> {
        if let Some(linked) = context.branches.iter().find(|branch| branch.is_linked()) {
            return Err(WorkspaceError::LinkedRepositoryIsReadOnly {
                repository: linked.repository.clone(),
            });
        }
        self.add_checked_context(context)
    }

    // TEMPORARY(legacy linked repositories): remove with `ContextCheckout::Linked`.
    /// Linked repositories only exist in projects imported before every
    /// repository got its own branch; tests build such a project with this.
    #[cfg(test)]
    pub fn add_legacy_context(self, context: Context) -> Result<Self, WorkspaceError> {
        self.add_checked_context(context)
    }

    fn add_checked_context(mut self, context: Context) -> Result<Self, WorkspaceError> {
        self.check_new_context(&context)?;
        let context = if context.is_default {
            Context {
                status: ContextStatus::Idle,
                is_static: true,
                base: None,
                ..context
            }
        } else {
            context
        };
        self.contexts.push(context);
        Ok(self)
    }

    fn check_new_context(&self, context: &Context) -> Result<(), WorkspaceError> {
        if context.is_default && self.default_context().is_some() {
            return Err(WorkspaceError::DefaultContextAlreadyExists);
        }
        if self
            .contexts
            .iter()
            .any(|existing| existing.name.is_same_as(&context.name))
        {
            return Err(WorkspaceError::DuplicateContextName {
                name: context.name.to_string(),
            });
        }
        if self
            .contexts
            .iter()
            .any(|existing| existing.folder.is_same_as(&context.folder))
        {
            return Err(WorkspaceError::DuplicateContextFolder {
                folder: context.folder.to_string(),
            });
        }
        if let Some(base) = &context.base {
            if self.find_context(base).is_none() {
                return Err(WorkspaceError::UnknownBaseContext { id: base.clone() });
            }
        }
        self.check_new_branches(&context.branches)
    }

    fn check_new_branches(&self, branches: &[ContextBranch]) -> Result<(), WorkspaceError> {
        for (index, branch) in branches.iter().enumerate() {
            if self.find_effective_repository(&branch.repository).is_none() {
                return Err(WorkspaceError::UnknownRepository {
                    id: branch.repository.clone(),
                });
            }
            if branches[..index]
                .iter()
                .any(|earlier| earlier.repository == branch.repository)
            {
                return Err(WorkspaceError::DuplicateContextBranch {
                    repository: branch.repository.clone(),
                });
            }
        }
        Ok(())
    }

    /// Removes a context. Contexts based on it no longer have a base context.
    pub fn delete_context(mut self, id: &ContextId) -> Result<Self, WorkspaceError> {
        if self.context(id)?.is_default {
            return Err(WorkspaceError::DefaultContextIsFixed);
        }
        self.contexts.retain(|context| &context.id != id);
        for context in &mut self.contexts {
            if context.base.as_ref() == Some(id) {
                context.base = None;
            }
        }
        Ok(self)
    }

    pub fn set_status(self, id: &ContextId, status: ContextStatus) -> Result<Self, WorkspaceError> {
        self.update_context(id, |context| {
            if context.is_static {
                return Err(WorkspaceError::StaticContextStatusLocked { id: id.clone() });
            }
            context.status = status;
            Ok(())
        })
    }

    pub fn set_static(self, id: &ContextId, is_static: bool) -> Result<Self, WorkspaceError> {
        self.update_context(id, |context| {
            if context.is_default {
                return Err(WorkspaceError::DefaultContextIsFixed);
            }
            context.is_static = is_static;
            Ok(())
        })
    }

    pub fn save_pull_request_draft(
        self,
        id: &ContextId,
        repository: &RepositoryId,
        draft: PullRequestDraft,
    ) -> Result<Self, WorkspaceError> {
        self.update_context(id, |context| {
            let branch = context.branch_for_mut(repository).ok_or_else(|| {
                WorkspaceError::UnknownContextBranch {
                    context: id.clone(),
                    repository: repository.clone(),
                }
            })?;
            if branch.is_linked() {
                return Err(WorkspaceError::LinkedRepositoryIsReadOnly {
                    repository: repository.clone(),
                });
            }
            branch.pull_request = Some(PullRequestLink { draft });
            Ok(())
        })
    }

    pub fn set_agent_prompt(
        self,
        id: &ContextId,
        prompt: Option<String>,
    ) -> Result<Self, WorkspaceError> {
        self.update_context(id, |context| {
            let agent = context.agent.take().unwrap_or_default();
            context.agent = Some(agent.with_prompt(prompt));
            Ok(())
        })
    }

    /// Remembers the agent's conversation so follow-up runs resume it.
    pub fn record_agent_conversation(
        self,
        id: &ContextId,
        conversation_id: String,
    ) -> Result<Self, WorkspaceError> {
        if conversation_id.trim().is_empty() {
            return Err(WorkspaceError::EmptyIdentifier);
        }
        self.update_context(id, |context| {
            let agent = context.agent.take().unwrap_or_default();
            context.agent = Some(agent.with_conversation(conversation_id));
            Ok(())
        })
    }

    /// Creates the default context, or gives it a branch for each repository
    /// it has none for, chosen by the default-branch rule among the branches
    /// available in that repository. Existing branches are kept; repositories
    /// without available branches are left out.
    pub fn repair_default_context(
        mut self,
        available_branches: &HashMap<RepositoryId, Vec<BranchName>>,
    ) -> Result<Self, WorkspaceError> {
        let current_branches = self
            .default_context()
            .map(|context| context.branches.clone())
            .unwrap_or_default();
        let missing_branches: Vec<ContextBranch> = self
            .effective_repositories()
            .iter()
            .filter(|repository| {
                !current_branches
                    .iter()
                    .any(|branch| branch.repository == repository.id)
            })
            .filter_map(|repository| {
                let branches = available_branches.get(&repository.id)?;
                let branch = choose_default_branch(branches)?;
                Some(ContextBranch::new(repository.id.clone(), branch.clone()))
            })
            .collect();
        match self.contexts.iter_mut().find(|context| context.is_default) {
            Some(default_context) => {
                default_context.branches.extend(missing_branches);
                Ok(self)
            }
            None => self.add_context(Context::new_default(missing_branches)),
        }
    }

    fn context(&self, id: &ContextId) -> Result<&Context, WorkspaceError> {
        self.find_context(id)
            .ok_or_else(|| WorkspaceError::UnknownContext { id: id.clone() })
    }

    fn update_context(
        mut self,
        id: &ContextId,
        change: impl FnOnce(&mut Context) -> Result<(), WorkspaceError>,
    ) -> Result<Self, WorkspaceError> {
        let context = self
            .contexts
            .iter_mut()
            .find(|context| &context.id == id)
            .ok_or_else(|| WorkspaceError::UnknownContext { id: id.clone() })?;
        change(context)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::domain::agent_work::AgentTool;
    use crate::workspace::domain::context_branch::ContextCheckout;
    use crate::workspace::domain::context_folder::ContextFolder;
    use crate::workspace::domain::context_name::ContextName;
    use crate::workspace::domain::context_origin::{ContextOrigin, TaskRef};
    use crate::workspace::domain::test_support::*;

    const API_ID: &str = "repo-1";
    const WEB_ID: &str = "repo-2";
    const FEATURE_BRANCH: &str = "feature/x";
    const FEATURE_NAME: &str = "My Feature";
    const PROMPT: &str = "Fix the login page";
    const CONVERSATION_ID: &str = "conv-123";

    fn api() -> Repository {
        repository(API_ID, "api", "./api")
    }

    fn web() -> Repository {
        repository(WEB_ID, "web", "./web")
    }

    fn feature_context() -> Context {
        Context {
            branches: vec![branch_on(API_ID, FEATURE_BRANCH)],
            ..context_named(FEATURE_NAME)
        }
    }

    /// A project with api and web, a default context on main, and `feature_context`.
    fn project_with_feature() -> (Project, ContextId) {
        let context = feature_context();
        let id = context.id.clone();
        let project = project_with(vec![api()])
            .add_context(Context::new_default(vec![branch_on(API_ID, "main")]))
            .unwrap()
            .add_context(context)
            .unwrap();
        (project, id)
    }

    fn default_id(project: &Project) -> ContextId {
        project.default_context().unwrap().id.clone()
    }

    fn available(entries: &[(&str, &[&str])]) -> HashMap<RepositoryId, Vec<BranchName>> {
        entries
            .iter()
            .map(|(repository, branches)| {
                let names = branches.iter().map(|name| branch_name(name)).collect();
                (repository_id(repository), names)
            })
            .collect()
    }

    fn branches_of(context: &Context) -> Vec<(&str, &str)> {
        context
            .branches
            .iter()
            .map(|branch| (branch.repository.as_str(), branch.branch.as_str()))
            .collect()
    }

    mod effective_repositories {
        use super::*;

        #[test]
        fn are_the_configured_repositories_when_there_are_some() {
            let project = project_with(vec![api()]);
            assert_eq!(project.effective_repositories().as_ref(), [api()]);
        }

        #[test]
        fn are_one_root_repository_for_a_single_repository_project() {
            let project = project_with(vec![]);
            let repositories = project.effective_repositories();
            assert_eq!(repositories.len(), 1);
            assert_eq!(repositories[0].id.as_str(), PROJECT_TAG);
            assert_eq!(repositories[0].name, PROJECT_NAME);
            assert_eq!(repositories[0].relative_path, ".");
        }

        #[test]
        fn finds_a_configured_repository_by_id() {
            let project = project_with(vec![api()]);
            let found = project.find_effective_repository(&repository_id(API_ID));
            assert_eq!(found.unwrap().name, "api");
        }

        #[test]
        fn finds_the_root_repository_by_the_project_tag() {
            let project = project_with(vec![]);
            let found = project.find_effective_repository(&repository_id(PROJECT_TAG));
            assert_eq!(found.unwrap().relative_path, ".");
        }

        #[test]
        fn finds_nothing_for_an_unknown_id() {
            let project = project_with(vec![api()]);
            assert!(project
                .find_effective_repository(&repository_id("missing"))
                .is_none());
        }
    }

    mod add_context {
        use super::*;

        #[test]
        fn adds_the_context() {
            let (project, id) = project_with_feature();
            assert_eq!(
                project.find_context(&id).unwrap().name.as_str(),
                FEATURE_NAME
            );
            assert_eq!(project.contexts().len(), 2);
        }

        #[test]
        fn refuses_a_name_already_used_ignoring_case() {
            let (project, _) = project_with_feature();
            let duplicate = Context {
                folder: ContextFolder::legacy(&ContextName::parse("elsewhere").unwrap()),
                ..context_named("MY FEATURE")
            };
            assert_eq!(
                project.add_context(duplicate),
                Err(WorkspaceError::DuplicateContextName {
                    name: "MY FEATURE".to_string()
                })
            );
        }

        #[test]
        fn refuses_a_name_whose_folder_is_already_used() {
            let (project, _) = project_with_feature();
            assert_eq!(
                project.add_context(context_named("my-feature")),
                Err(WorkspaceError::DuplicateContextFolder {
                    folder: "my-feature".to_string()
                })
            );
        }

        #[test]
        fn refuses_a_folder_matching_a_legacy_folder_ignoring_case() {
            let legacy = Context {
                folder: ContextFolder::legacy(&ContextName::parse("Release").unwrap()),
                ..context_named("Release")
            };
            let project = project_with(vec![api()]).add_context(legacy).unwrap();
            assert_eq!(
                project.add_context(context_named("release!")),
                Err(WorkspaceError::DuplicateContextFolder {
                    folder: "release".to_string()
                })
            );
        }

        #[test]
        fn keeps_a_known_base_context() {
            let (project, base) = project_with_feature();
            let based = Context {
                base: Some(base.clone()),
                ..context_named("Follow up")
            };
            let based_id = based.id.clone();
            let project = project.add_context(based).unwrap();
            assert_eq!(project.find_context(&based_id).unwrap().base, Some(base));
        }

        #[test]
        fn refuses_an_unknown_base_context() {
            let unknown = ContextId::parse("missing").unwrap();
            let context = Context {
                base: Some(unknown.clone()),
                ..context_named(FEATURE_NAME)
            };
            assert_eq!(
                project_with(vec![api()]).add_context(context),
                Err(WorkspaceError::UnknownBaseContext { id: unknown })
            );
        }

        #[test]
        fn refuses_a_branch_on_an_unknown_repository() {
            let context = Context {
                branches: vec![branch_on("missing", FEATURE_BRANCH)],
                ..context_named(FEATURE_NAME)
            };
            assert_eq!(
                project_with(vec![api()]).add_context(context),
                Err(WorkspaceError::UnknownRepository {
                    id: repository_id("missing")
                })
            );
        }

        #[test]
        fn refuses_two_branches_on_one_repository() {
            let context = Context {
                branches: vec![
                    branch_on(API_ID, FEATURE_BRANCH),
                    branch_on(API_ID, "fix/y"),
                ],
                ..context_named(FEATURE_NAME)
            };
            assert_eq!(
                project_with(vec![api()]).add_context(context),
                Err(WorkspaceError::DuplicateContextBranch {
                    repository: repository_id(API_ID)
                })
            );
        }

        #[test]
        fn refuses_a_linked_repository() {
            let linked = ContextBranch {
                checkout: ContextCheckout::Linked,
                ..branch_on(API_ID, FEATURE_BRANCH)
            };
            let context = Context {
                branches: vec![linked],
                ..context_named(FEATURE_NAME)
            };
            assert_eq!(
                project_with(vec![api()]).add_context(context),
                Err(WorkspaceError::LinkedRepositoryIsReadOnly {
                    repository: repository_id(API_ID)
                })
            );
        }

        #[test]
        fn accepts_a_linked_repository_from_an_old_project_file() {
            let linked = ContextBranch {
                checkout: ContextCheckout::Linked,
                ..branch_on(API_ID, FEATURE_BRANCH)
            };
            let context = Context {
                branches: vec![linked],
                ..context_named(FEATURE_NAME)
            };
            let id = context.id.clone();
            let project = project_with(vec![api()])
                .add_legacy_context(context)
                .unwrap();
            assert!(project.find_context(&id).unwrap().branches[0].is_linked());
        }

        #[test]
        fn refuses_a_duplicate_name_from_an_old_project_file() {
            let (project, _) = project_with_feature();
            assert_eq!(
                project.add_legacy_context(context_named(FEATURE_NAME)),
                Err(WorkspaceError::DuplicateContextName {
                    name: FEATURE_NAME.to_string()
                })
            );
        }

        #[test]
        fn refuses_a_second_default_context() {
            let (project, _) = project_with_feature();
            let second_default = Context {
                name: ContextName::parse("other default").unwrap(),
                folder: ContextFolder::legacy(&ContextName::parse("other").unwrap()),
                ..Context::new_default(vec![])
            };
            assert_eq!(
                project.add_context(second_default),
                Err(WorkspaceError::DefaultContextAlreadyExists)
            );
        }

        #[test]
        fn makes_an_added_default_context_idle_and_static() {
            let loose_default = Context {
                status: ContextStatus::Review,
                is_static: false,
                ..Context::new_default(vec![])
            };
            let project = project_with(vec![]).add_context(loose_default).unwrap();
            let default_context = project.default_context().unwrap();
            assert_eq!(default_context.status, ContextStatus::Idle);
            assert!(default_context.is_static);
        }

        #[test]
        fn keeps_the_settings() {
            let project = project_with(vec![api()]);
            let settings_before = (project.branch_naming().clone(), project.symlinks().to_vec());
            let project = project.add_context(feature_context()).unwrap();
            assert_eq!(
                (project.branch_naming().clone(), project.symlinks().to_vec()),
                settings_before
            );
        }
    }

    mod delete_context {
        use super::*;

        #[test]
        fn removes_the_context() {
            let (project, id) = project_with_feature();
            let project = project.delete_context(&id).unwrap();
            assert!(project.find_context(&id).is_none());
        }

        #[test]
        fn refuses_the_default_context() {
            let (project, _) = project_with_feature();
            let id = default_id(&project);
            assert_eq!(
                project.delete_context(&id),
                Err(WorkspaceError::DefaultContextIsFixed)
            );
        }

        #[test]
        fn refuses_an_unknown_context() {
            let (project, _) = project_with_feature();
            let unknown = ContextId::parse("missing").unwrap();
            assert_eq!(
                project.delete_context(&unknown),
                Err(WorkspaceError::UnknownContext { id: unknown })
            );
        }

        #[test]
        fn clears_the_base_of_contexts_based_on_it() {
            let (project, base) = project_with_feature();
            let based = Context {
                base: Some(base.clone()),
                ..context_named("Follow up")
            };
            let based_id = based.id.clone();
            let project = project
                .add_context(based)
                .unwrap()
                .delete_context(&base)
                .unwrap();
            assert_eq!(project.find_context(&based_id).unwrap().base, None);
        }

        #[test]
        fn adding_then_deleting_gives_back_the_same_project() {
            let project = project_with(vec![api()]);
            let context = feature_context();
            let id = context.id.clone();
            let round_trip = project
                .clone()
                .add_context(context)
                .unwrap()
                .delete_context(&id);
            assert_eq!(round_trip, Ok(project));
        }
    }

    mod set_status {
        use super::*;

        #[test]
        fn moves_a_non_static_context_to_any_status() {
            let (project, id) = project_with_feature();
            let project = project.set_status(&id, ContextStatus::Review).unwrap();
            let project = project.set_status(&id, ContextStatus::Idle).unwrap();
            assert_eq!(
                project.find_context(&id).unwrap().status,
                ContextStatus::Idle
            );
        }

        #[test]
        fn refuses_a_static_context() {
            let (project, id) = project_with_feature();
            let project = project.set_static(&id, true).unwrap();
            assert_eq!(
                project.set_status(&id, ContextStatus::Done),
                Err(WorkspaceError::StaticContextStatusLocked { id })
            );
        }

        #[test]
        fn refuses_the_default_context() {
            let (project, _) = project_with_feature();
            let id = default_id(&project);
            assert_eq!(
                project.set_status(&id, ContextStatus::Done),
                Err(WorkspaceError::StaticContextStatusLocked { id })
            );
        }

        #[test]
        fn keeps_the_task_the_context_was_created_from() {
            let task = TaskRef {
                connection_id: "teamwork:acme.teamwork.com".to_string(),
                task_id: "101".to_string(),
            };
            let context = Context {
                origin: ContextOrigin::FromTask(task.clone()),
                ..context_named("Task")
            };
            let id = context.id.clone();
            let project = project_with(vec![])
                .add_context(context)
                .unwrap()
                .set_status(&id, ContextStatus::Review)
                .unwrap();
            assert!(project.find_context(&id).unwrap().is_created_from(&task));
        }
    }

    mod set_static {
        use super::*;

        #[test]
        fn marks_a_context_static_and_back() {
            let (project, id) = project_with_feature();
            let project = project.set_static(&id, true).unwrap();
            assert!(project.find_context(&id).unwrap().is_static);
            let project = project.set_static(&id, false).unwrap();
            assert!(!project.find_context(&id).unwrap().is_static);
        }

        #[test]
        fn refuses_the_default_context() {
            let (project, _) = project_with_feature();
            let id = default_id(&project);
            assert_eq!(
                project.set_static(&id, false),
                Err(WorkspaceError::DefaultContextIsFixed)
            );
        }
    }

    mod save_pull_request_draft {
        use super::*;

        fn draft() -> PullRequestDraft {
            PullRequestDraft {
                title: "Add login".to_string(),
                description: "Adds the login page".to_string(),
            }
        }

        #[test]
        fn saves_the_draft_on_the_context_branch() {
            let (project, id) = project_with_feature();
            let project = project
                .save_pull_request_draft(&id, &repository_id(API_ID), draft())
                .unwrap();
            let branch = project
                .find_context(&id)
                .unwrap()
                .branch_for(&repository_id(API_ID))
                .unwrap();
            assert_eq!(
                branch.pull_request,
                Some(PullRequestLink { draft: draft() })
            );
        }

        #[test]
        fn refuses_a_repository_the_context_has_no_branch_for() {
            let (project, id) = project_with_feature();
            assert_eq!(
                project.save_pull_request_draft(&id, &repository_id(WEB_ID), draft()),
                Err(WorkspaceError::UnknownContextBranch {
                    context: id,
                    repository: repository_id(WEB_ID)
                })
            );
        }
    }

    mod agent_work {
        use super::*;

        #[test]
        fn a_new_context_has_no_agent_work() {
            let (project, id) = project_with_feature();
            assert_eq!(project.find_context(&id).unwrap().agent, None);
        }

        #[test]
        fn sets_the_prompt() {
            let (project, id) = project_with_feature();
            let project = project
                .set_agent_prompt(&id, Some(PROMPT.to_string()))
                .unwrap();
            let agent = project.find_context(&id).unwrap().agent.clone().unwrap();
            assert_eq!(agent.prompt.as_deref(), Some(PROMPT));
        }

        #[test]
        fn records_the_conversation_keeping_the_prompt() {
            let (project, id) = project_with_feature();
            let project = project
                .set_agent_prompt(&id, Some(PROMPT.to_string()))
                .unwrap()
                .record_agent_conversation(&id, CONVERSATION_ID.to_string())
                .unwrap();
            let agent = project.find_context(&id).unwrap().agent.clone().unwrap();
            assert_eq!(agent.prompt.as_deref(), Some(PROMPT));
            assert_eq!(
                agent.tool,
                AgentTool::Kiro {
                    conversation_id: Some(CONVERSATION_ID.to_string())
                }
            );
        }

        #[test]
        fn refuses_a_blank_conversation() {
            let (project, id) = project_with_feature();
            assert_eq!(
                project.record_agent_conversation(&id, " ".to_string()),
                Err(WorkspaceError::EmptyIdentifier)
            );
        }

        #[test]
        fn refuses_an_unknown_context() {
            let (project, _) = project_with_feature();
            let unknown = ContextId::parse("missing").unwrap();
            assert_eq!(
                project.record_agent_conversation(&unknown, CONVERSATION_ID.to_string()),
                Err(WorkspaceError::UnknownContext { id: unknown })
            );
        }
    }

    mod repair_default_context {
        use super::*;

        #[test]
        fn creates_the_default_context_on_each_default_branch() {
            let project = project_with(vec![api(), web()])
                .repair_default_context(&available(&[
                    (API_ID, &["develop", "main"]),
                    (WEB_ID, &["develop", "master"]),
                ]))
                .unwrap();
            let default_context = project.default_context().unwrap();
            assert_eq!(
                branches_of(default_context),
                [(API_ID, "main"), (WEB_ID, "master")]
            );
            assert_eq!(default_context.status, ContextStatus::Idle);
            assert!(default_context.is_static);
        }

        #[test]
        fn uses_the_root_repository_of_a_single_repository_project() {
            let project = project_with(vec![])
                .repair_default_context(&available(&[(PROJECT_TAG, &["trunk"])]))
                .unwrap();
            assert_eq!(
                branches_of(project.default_context().unwrap()),
                [(PROJECT_TAG, "trunk")]
            );
        }

        #[test]
        fn adds_missing_repositories_and_keeps_existing_branches() {
            let project = project_with(vec![api(), web()])
                .add_context(Context::new_default(vec![branch_on(API_ID, "develop")]))
                .unwrap()
                .repair_default_context(&available(&[(API_ID, &["main"]), (WEB_ID, &["main"])]))
                .unwrap();
            assert_eq!(
                branches_of(project.default_context().unwrap()),
                [(API_ID, "develop"), (WEB_ID, "main")]
            );
        }

        #[test]
        fn leaves_out_repositories_without_available_branches() {
            let project = project_with(vec![api(), web()])
                .repair_default_context(&available(&[(API_ID, &["main"]), (WEB_ID, &[])]))
                .unwrap();
            assert_eq!(
                branches_of(project.default_context().unwrap()),
                [(API_ID, "main")]
            );
        }

        #[test]
        fn changes_nothing_when_the_default_context_is_complete() {
            let (project, _) = project_with_feature();
            let repaired = project
                .clone()
                .repair_default_context(&available(&[(API_ID, &["master"])]))
                .unwrap();
            assert_eq!(repaired, project);
        }
    }

    mod settings {
        use super::*;

        const BLANK: &str = "  ";

        fn link() -> RemoteProjectLink {
            RemoteProjectLink {
                provider_kind: "teamwork".to_string(),
                connection_id: "teamwork:acme.teamwork.com".to_string(),
                remote_project_id: "42".to_string(),
                remote_project_name: "Website".to_string(),
                remote_project_url: Some("https://acme.teamwork.com/app/projects/42".to_string()),
                linked_at: "2024-01-15T10:00:00.000Z".to_string(),
            }
        }

        fn linked_project() -> Project {
            let settings = ProjectSettings {
                icon: Some("rocket".to_string()),
                symlinks: vec!["node_modules".to_string()],
                remote_project_links: vec![link()],
                branch_naming: BranchNamingPolicy::new(&["ops"], false),
                ..settings(vec![api()])
            };
            Project::new(ProjectTag::parse(PROJECT_TAG).unwrap(), settings)
                .add_context(feature_context())
                .unwrap()
        }

        #[test]
        fn a_new_project_uses_the_default_branch_naming_policy() {
            assert_eq!(
                project_with(vec![]).branch_naming(),
                &BranchNamingPolicy::default()
            );
        }

        #[test]
        fn updating_replaces_the_settings_and_keeps_the_contexts() {
            let project = linked_project();
            let contexts = project.contexts().to_vec();
            let updated = project.update_settings(settings(vec![api()])).unwrap();
            assert!(updated.remote_project_links().is_empty());
            assert_eq!(updated.icon(), None);
            assert_eq!(updated.contexts(), contexts);
            assert_eq!(updated.tag().as_str(), PROJECT_TAG);
        }

        #[test]
        fn refuses_a_blank_name_or_folder() {
            let cases = [
                (
                    ProjectSettings {
                        name: BLANK.to_string(),
                        ..settings(vec![])
                    },
                    WorkspaceError::EmptyProjectName,
                ),
                (
                    ProjectSettings {
                        path: BLANK.to_string(),
                        ..settings(vec![])
                    },
                    WorkspaceError::EmptyProjectPath,
                ),
            ];
            for (blank_settings, error) in cases {
                assert_eq!(
                    project_with(vec![]).update_settings(blank_settings),
                    Err(error)
                );
            }
        }

        #[test]
        fn refuses_two_repositories_with_one_id() {
            let twin = repository(API_ID, "api copy", "./api-copy");
            assert_eq!(
                Project::create(
                    ProjectTag::parse(PROJECT_TAG).unwrap(),
                    settings(vec![api(), twin])
                ),
                Err(WorkspaceError::DuplicateRepository {
                    id: repository_id(API_ID)
                })
            );
        }

        #[test]
        fn drops_blank_symlinks_icon_and_post_checkout_command() {
            let blank_command = Repository {
                post_checkout_command: Some(BLANK.to_string()),
                ..api()
            };
            let updated = project_with(vec![])
                .update_settings(ProjectSettings {
                    icon: Some(BLANK.to_string()),
                    symlinks: vec![BLANK.to_string(), ".env".to_string()],
                    ..settings(vec![blank_command])
                })
                .unwrap();
            assert_eq!(updated.icon(), None);
            assert_eq!(updated.symlinks(), [".env"]);
            assert_eq!(updated.repositories()[0].post_checkout_command, None);
        }

        #[test]
        fn round_trips_through_serialization() {
            let project = linked_project();
            let json = serde_json::to_string(&project).unwrap();
            assert_eq!(serde_json::from_str::<Project>(&json).unwrap(), project);
        }

        #[test]
        fn reads_a_project_saved_without_links_or_branch_naming() {
            let saved = serde_json::json!({
                "tag": PROJECT_TAG,
                "name": PROJECT_NAME,
                "path": PROJECT_PATH,
                "icon": null,
                "repositories": [],
                "contexts": [],
                "symlinks": [],
            });
            let project: Project = serde_json::from_value(saved).unwrap();
            assert!(project.remote_project_links().is_empty());
            assert_eq!(project.branch_naming(), &BranchNamingPolicy::default());
        }

        #[test]
        fn serializes_absent_values_as_null() {
            let json = serde_json::to_value(project_with(vec![])).unwrap();
            assert_eq!(json["icon"], serde_json::Value::Null);
        }
    }
}
