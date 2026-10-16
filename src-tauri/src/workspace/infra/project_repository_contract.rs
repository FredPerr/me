//! Behaviour every `ProjectRepository` adapter shares. An adapter's tests run
//! it with `project_repository_contract_tests!(with_repository)`, where
//! `with_repository` runs a test against a fresh, empty repository.

use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use crate::workspace::domain::context::Context;
use crate::workspace::domain::context_branch::PullRequestDraft;
use crate::workspace::domain::context_origin::{ContextOrigin, TaskRef};
use crate::workspace::domain::context_status::ContextStatus;
use crate::workspace::domain::error::WorkspaceError;
use crate::workspace::domain::identifiers::ContextId;
use crate::workspace::domain::ports::{ProjectRepository, UpdateError};
use crate::workspace::domain::project::{Project, ProjectSettings};
use crate::workspace::domain::project_tag::ProjectTag;
use crate::workspace::domain::test_support::{
    branch_name, branch_on, context_named, project_with, repository, repository_id, settings,
};

const API_REPOSITORY: &str = "api";
const WEB_REPOSITORY: &str = "web";
const MAIN_BRANCH: &str = "main";
const LOGIN_CONTEXT: &str = "Login page";
const LOGIN_BRANCH: &str = "feature/login";
const PINNED_CONTEXT: &str = "Pinned notes";
const PINNED_BRANCH: &str = "notes";
const CONNECTION_ID: &str = "teamwork:acme.teamwork.com";
const TASK_ID: &str = "101";
const DRAFT_TITLE: &str = "Add the login page";
const DRAFT_DESCRIPTION: &str = "Adds the login form.";
const AGENT_PROMPT: &str = "Build the login page";
const CONVERSATION_ID: &str = "conversation-1";
const RENAMED_PROJECT: &str = "Renamed project";
const UNKNOWN_TAG: &str = "unknown";
const ALPHA_TAG: &str = "alpha";
const BETA_TAG: &str = "beta";
const GAMMA_TAG: &str = "gamma";
const CONCURRENT_CONTEXT_COUNT: usize = 8;
const TIME_INSIDE_A_CHANGE: Duration = Duration::from_millis(5);

pub fn project_tagged(tag: &str) -> Project {
    Project::new(ProjectTag::parse(tag).unwrap(), settings(vec![]))
}

fn with_default_context(project: Project) -> Project {
    let available_branches = project
        .effective_repositories()
        .iter()
        .map(|repository| (repository.id.clone(), vec![branch_name(MAIN_BRANCH)]))
        .collect::<HashMap<_, _>>();
    project.repair_default_context(&available_branches).unwrap()
}

fn context_id_named(project: &Project, name: &str) -> ContextId {
    project
        .contexts()
        .iter()
        .find(|context| context.name.as_str() == name)
        .unwrap()
        .id
        .clone()
}

fn default_context_id(project: &Project) -> ContextId {
    project.default_context().unwrap().id.clone()
}

/// Two repositories, the default context, a context created from a task with
/// a pull request draft and agent work, and a static context under review.
pub fn project_with_contexts() -> Project {
    let project = with_default_context(project_with(vec![
        repository(API_REPOSITORY, API_REPOSITORY, API_REPOSITORY),
        repository(WEB_REPOSITORY, WEB_REPOSITORY, WEB_REPOSITORY),
    ]));
    let login_context = Context {
        base: Some(default_context_id(&project)),
        branches: vec![
            branch_on(API_REPOSITORY, LOGIN_BRANCH),
            branch_on(WEB_REPOSITORY, LOGIN_BRANCH),
        ],
        origin: ContextOrigin::FromTask(TaskRef {
            connection_id: CONNECTION_ID.to_string(),
            task_id: TASK_ID.to_string(),
        }),
        ..context_named(LOGIN_CONTEXT)
    };
    let pinned_context = Context {
        branches: vec![branch_on(API_REPOSITORY, PINNED_BRANCH)],
        ..context_named(PINNED_CONTEXT)
    };
    let project = project
        .add_context(login_context)
        .unwrap()
        .add_context(pinned_context)
        .unwrap();
    let login_id = context_id_named(&project, LOGIN_CONTEXT);
    let pinned_id = context_id_named(&project, PINNED_CONTEXT);
    project
        .set_status(&login_id, ContextStatus::InProgress)
        .unwrap()
        .save_pull_request_draft(
            &login_id,
            &repository_id(API_REPOSITORY),
            PullRequestDraft {
                title: DRAFT_TITLE.to_string(),
                description: DRAFT_DESCRIPTION.to_string(),
            },
        )
        .unwrap()
        .set_agent_prompt(&login_id, Some(AGENT_PROMPT.to_string()))
        .unwrap()
        .record_agent_conversation(&login_id, CONVERSATION_ID.to_string())
        .unwrap()
        .set_status(&pinned_id, ContextStatus::Review)
        .unwrap()
        .set_static(&pinned_id, true)
        .unwrap()
}

fn project_with_ready_contexts(count: usize) -> Project {
    (0..count).fold(
        with_default_context(project_with(vec![])),
        |project, index| {
            project
                .add_context(context_named(&format!("Context {index}")))
                .unwrap()
        },
    )
}

fn tags_of(projects: &[Project]) -> Vec<&str> {
    projects
        .iter()
        .map(|project| project.tag().as_str())
        .collect()
}

pub fn saves_and_finds_a_project_with_its_contexts(repository: &dyn ProjectRepository) {
    let project = project_with_contexts();

    repository.save(&project).unwrap();

    assert_eq!(repository.find(project.tag()).unwrap(), Some(project));
}

pub fn finds_nothing_for_an_unknown_tag(repository: &dyn ProjectRepository) {
    let found = repository
        .find(&ProjectTag::parse(UNKNOWN_TAG).unwrap())
        .unwrap();

    assert_eq!(found, None);
}

pub fn save_replaces_the_project_with_the_same_tag(repository: &dyn ProjectRepository) {
    let project = project_with_contexts();
    repository.save(&project).unwrap();
    let renamed = project.clone().update_settings(ProjectSettings {
        name: RENAMED_PROJECT.to_string(),
        ..settings(project.repositories().to_vec())
    });

    repository.save(&renamed).unwrap();

    assert_eq!(repository.list().unwrap().projects, vec![renamed]);
}

pub fn lists_projects_ordered_by_tag(repository: &dyn ProjectRepository) {
    for tag in [GAMMA_TAG, ALPHA_TAG, BETA_TAG] {
        repository.save(&project_tagged(tag)).unwrap();
    }

    let listing = repository.list().unwrap();

    assert_eq!(
        tags_of(&listing.projects),
        vec![ALPHA_TAG, BETA_TAG, GAMMA_TAG]
    );
    assert!(listing.unreadable.is_empty());
}

pub fn delete_removes_the_project_and_is_idempotent(repository: &dyn ProjectRepository) {
    let project = project_with_contexts();
    repository.save(&project).unwrap();

    repository.delete(project.tag()).unwrap();
    repository.delete(project.tag()).unwrap();

    assert_eq!(repository.find(project.tag()).unwrap(), None);
}

pub fn update_applies_the_change_and_saves_it(repository: &dyn ProjectRepository) {
    let project = project_with_contexts();
    repository.save(&project).unwrap();
    let login_id = context_id_named(&project, LOGIN_CONTEXT);

    let updated = repository
        .update(
            project.tag(),
            Box::new(|stored| stored.set_status(&login_id, ContextStatus::Done)),
        )
        .unwrap();

    assert_eq!(
        updated.find_context(&login_id).unwrap().status,
        ContextStatus::Done
    );
    assert_eq!(repository.find(project.tag()).unwrap(), Some(updated));
}

pub fn update_writes_nothing_when_the_change_is_rejected(repository: &dyn ProjectRepository) {
    let project = project_with_contexts();
    repository.save(&project).unwrap();
    let login_id = context_id_named(&project, LOGIN_CONTEXT);
    let default_id = default_context_id(&project);

    let result = repository.update(
        project.tag(),
        Box::new(|stored| {
            stored
                .set_status(&login_id, ContextStatus::Done)?
                .delete_context(&default_id)
        }),
    );

    assert_eq!(
        result,
        Err(UpdateError::Rejected(WorkspaceError::DefaultContextIsFixed))
    );
    assert_eq!(repository.find(project.tag()).unwrap(), Some(project));
}

pub fn update_reports_a_missing_project(repository: &dyn ProjectRepository) {
    let tag = ProjectTag::parse(UNKNOWN_TAG).unwrap();

    let result = repository.update(&tag, Box::new(Ok));

    assert_eq!(result, Err(UpdateError::NotFound { tag }));
}

/// Each thread moves a different context; a lost update would leave one ready.
pub fn concurrent_updates_all_land(repository: &dyn ProjectRepository) {
    let project = project_with_ready_contexts(CONCURRENT_CONTEXT_COUNT);
    repository.save(&project).unwrap();
    let moved_ids: Vec<ContextId> = project
        .contexts()
        .iter()
        .filter(|context| !context.is_default)
        .map(|context| context.id.clone())
        .collect();

    thread::scope(|scope| {
        for id in &moved_ids {
            scope.spawn(|| {
                repository
                    .update(
                        project.tag(),
                        Box::new(|stored| {
                            thread::sleep(TIME_INSIDE_A_CHANGE);
                            stored.set_status(id, ContextStatus::Review)
                        }),
                    )
                    .unwrap();
            });
        }
    });

    let stored = repository.find(project.tag()).unwrap().unwrap();
    for id in &moved_ids {
        assert_eq!(
            stored.find_context(id).unwrap().status,
            ContextStatus::Review
        );
    }
}

/// Generates one `#[test]` per contract behaviour, each run through
/// `$with_repository: fn(fn(&dyn ProjectRepository))`.
macro_rules! project_repository_contract_tests {
    ($with_repository:ident) => {
        $crate::workspace::infra::project_repository_contract::project_repository_contract_tests!(
            @tests $with_repository;
            saves_and_finds_a_project_with_its_contexts,
            finds_nothing_for_an_unknown_tag,
            save_replaces_the_project_with_the_same_tag,
            lists_projects_ordered_by_tag,
            delete_removes_the_project_and_is_idempotent,
            update_applies_the_change_and_saves_it,
            update_writes_nothing_when_the_change_is_rejected,
            update_reports_a_missing_project,
            concurrent_updates_all_land,
        );
    };
    (@tests $with_repository:ident; $($behaviour:ident),* $(,)?) => {
        mod contract {
            $(
                #[test]
                fn $behaviour() {
                    super::$with_repository(
                        $crate::workspace::infra::project_repository_contract::$behaviour,
                    );
                }
            )*
        }
    };
}

pub(crate) use project_repository_contract_tests;
