use serde::{Deserialize, Serialize};

use super::agent_work::AgentWork;
use super::context_branch::ContextBranch;
use super::context_folder::ContextFolder;
use super::context_name::ContextName;
use super::context_origin::{ContextOrigin, TaskRef};
use super::context_status::ContextStatus;
use super::error::WorkspaceError;
use super::identifiers::{ContextId, RepositoryId};

pub const DEFAULT_CONTEXT_NAME: &str = "default";

/// A named set of checkouts, one branch per repository of a project.
///
/// Fields are public so a context can be built freely; it only becomes part
/// of a Project through `Project::add_context`, which checks the rules, and is
/// only changed through the Project's methods.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub id: ContextId,
    pub name: ContextName,
    pub folder: ContextFolder,
    pub status: ContextStatus,
    pub is_default: bool,
    pub is_static: bool,
    pub base: Option<ContextId>,
    pub branches: Vec<ContextBranch>,
    pub origin: ContextOrigin,
    pub agent: Option<AgentWork>,
}

impl Context {
    /// A new, ready, non-static context made by hand, in the folder named after
    /// the slug of its name.
    pub fn new(name: ContextName, branches: Vec<ContextBranch>) -> Result<Self, WorkspaceError> {
        Ok(Self {
            id: ContextId::generate(),
            folder: ContextFolder::from_name(&name)?,
            name,
            status: ContextStatus::default(),
            is_default: false,
            is_static: false,
            base: None,
            branches,
            origin: ContextOrigin::default(),
            agent: None,
        })
    }

    /// The project's own main checkouts: always idle and static.
    pub fn new_default(branches: Vec<ContextBranch>) -> Self {
        let name = ContextName::parse(DEFAULT_CONTEXT_NAME).expect("the default name is valid");
        Self {
            id: ContextId::generate(),
            folder: ContextFolder::from_name(&name).expect("the default name has a slug"),
            name,
            status: ContextStatus::Idle,
            is_default: true,
            is_static: true,
            base: None,
            branches,
            origin: ContextOrigin::default(),
            agent: None,
        }
    }

    pub fn branch_for(&self, repository: &RepositoryId) -> Option<&ContextBranch> {
        self.branches
            .iter()
            .find(|branch| &branch.repository == repository)
    }

    pub fn is_created_from(&self, task: &TaskRef) -> bool {
        matches!(&self.origin, ContextOrigin::FromTask(origin) if origin == task)
    }

    pub(super) fn branch_for_mut(
        &mut self,
        repository: &RepositoryId,
    ) -> Option<&mut ContextBranch> {
        self.branches
            .iter_mut()
            .find(|branch| &branch.repository == repository)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::domain::test_support::{branch_on, context_named, repository_id};

    const REPOSITORY: &str = "repository-1";
    const OTHER_REPOSITORY: &str = "repository-2";
    const BRANCH: &str = "feature/x";

    fn task(task_id: &str) -> TaskRef {
        TaskRef {
            connection_id: "teamwork:acme.teamwork.com".to_string(),
            task_id: task_id.to_string(),
        }
    }

    #[test]
    fn a_new_context_is_ready_non_static_and_made_by_hand() {
        let context = context_named("My Feature");
        assert_eq!(context.status, ContextStatus::Ready);
        assert!(!context.is_static);
        assert!(!context.is_default);
        assert_eq!(context.origin, ContextOrigin::Manual);
        assert_eq!(context.agent, None);
    }

    #[test]
    fn a_new_context_uses_the_slug_of_its_name_as_folder() {
        assert_eq!(
            context_named("Ajout d'un pipeline").folder.as_str(),
            "ajout-d-un-pipeline"
        );
    }

    #[test]
    fn the_default_context_is_idle_and_static() {
        let context = Context::new_default(vec![]);
        assert_eq!(context.status, ContextStatus::Idle);
        assert!(context.is_static);
        assert!(context.is_default);
        assert_eq!(context.name.as_str(), DEFAULT_CONTEXT_NAME);
    }

    #[test]
    fn finds_its_branch_for_a_repository() {
        let context = Context {
            branches: vec![branch_on(REPOSITORY, BRANCH)],
            ..context_named("My Feature")
        };
        let found = context.branch_for(&repository_id(REPOSITORY)).unwrap();
        assert_eq!(found.branch.as_str(), BRANCH);
        assert_eq!(context.branch_for(&repository_id(OTHER_REPOSITORY)), None);
    }

    #[test]
    fn recognizes_the_task_it_was_created_from() {
        let context = Context {
            origin: ContextOrigin::FromTask(task("101")),
            ..context_named("Task")
        };
        assert!(context.is_created_from(&task("101")));
        assert!(!context.is_created_from(&task("102")));
    }

    #[test]
    fn has_no_expanded_flag_when_serialized() {
        let json = serde_json::to_value(context_named("My Feature")).unwrap();
        assert!(json.get("expanded").is_none(), "{json}");
    }

    #[test]
    fn round_trips_through_serialization() {
        let context = Context {
            is_static: true,
            origin: ContextOrigin::FromTask(task("101")),
            agent: Some(AgentWork::default().with_conversation("conv-123".to_string())),
            branches: vec![branch_on(REPOSITORY, BRANCH)],
            ..context_named("My Feature")
        };
        let json = serde_json::to_string(&context).unwrap();
        assert_eq!(serde_json::from_str::<Context>(&json).unwrap(), context);
    }
}
