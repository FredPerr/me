use std::collections::HashMap;

use super::*;
use crate::git::domain::branch_name::BranchName;
use crate::workspace::application::error::CheckoutHolder;
use crate::workspace::domain::error::PrefixRejection;
use crate::workspace::domain::ports::{CheckoutFilesError, StorageError, UpdateError};
use crate::workspace::domain::test_support::{
    branch_name, branch_on, context_named, project_with, repository, repository_id, PROJECT_PATH,
    PROJECT_TAG,
};

const REPOSITORY_NAME: &str = "api";
const BRANCH: &str = "feature/login";
const CONTEXT_NAME: &str = "Login page";
const STORAGE_REASON: &str = "disk full";
const TASK_ID: &str = "101";
const CONNECTION_ID: &str = "teamwork:acme.teamwork.com";

fn kind_of(error: impl Into<WorkspaceCommandError>) -> WorkspaceErrorKind {
    error.into().kind
}

fn context_id() -> ContextId {
    ContextId::parse("context-1").unwrap()
}

fn branch() -> BranchName {
    branch_name(BRANCH)
}

fn git_failure(error: GitError) -> WorkspaceServiceError {
    WorkspaceServiceError::git(REPOSITORY_NAME, error)
}

mod error_kinds {
    use super::*;

    #[test]
    fn every_input_error_is_invalid_input() {
        let input_errors = [
            WorkspaceError::EmptyProjectTag,
            WorkspaceError::EmptyIdentifier,
            WorkspaceError::EmptyContextName,
            WorkspaceError::EmptyContextFolder { name: "!!!".into() },
            WorkspaceError::DuplicateContextBranch {
                repository: repository_id(REPOSITORY_NAME),
            },
            WorkspaceError::InvalidBranchName {
                name: "a..b".into(),
                reason: "contains \"..\"",
            },
            WorkspaceError::EmptyBranchName {
                source: "!!!".into(),
            },
            WorkspaceError::InvalidPrefix {
                prefix: "".into(),
                reason: PrefixRejection::Empty,
            },
            WorkspaceError::UnknownPrefix {
                prefix: "hotfix".into(),
            },
            WorkspaceError::PrefixRequired,
            WorkspaceError::EmptyProjectName,
            WorkspaceError::EmptyProjectPath,
            WorkspaceError::DuplicateRepository {
                id: repository_id(REPOSITORY_NAME),
            },
        ];
        for error in input_errors {
            assert_eq!(kind_of(error), WorkspaceErrorKind::InvalidInput);
        }
    }

    #[test]
    fn every_domain_refusal_has_its_kind() {
        let cases = [
            (
                WorkspaceError::DuplicateContextName {
                    name: CONTEXT_NAME.into(),
                },
                WorkspaceErrorKind::DuplicateContextName,
            ),
            (
                WorkspaceError::DuplicateContextFolder {
                    folder: "login-page".into(),
                },
                WorkspaceErrorKind::DuplicateContextFolder,
            ),
            (
                WorkspaceError::UnknownContext { id: context_id() },
                WorkspaceErrorKind::UnknownContext,
            ),
            (
                WorkspaceError::UnknownBaseContext { id: context_id() },
                WorkspaceErrorKind::UnknownContext,
            ),
            (
                WorkspaceError::UnknownContextBranch {
                    context: context_id(),
                    repository: repository_id(REPOSITORY_NAME),
                },
                WorkspaceErrorKind::UnknownContext,
            ),
            (
                WorkspaceError::UnknownRepository {
                    id: repository_id(REPOSITORY_NAME),
                },
                WorkspaceErrorKind::UnknownRepository,
            ),
            (
                WorkspaceError::DefaultContextAlreadyExists,
                WorkspaceErrorKind::DefaultContextIsFixed,
            ),
            (
                WorkspaceError::DefaultContextIsFixed,
                WorkspaceErrorKind::DefaultContextIsFixed,
            ),
            (
                WorkspaceError::StaticContextStatusLocked { id: context_id() },
                WorkspaceErrorKind::StaticContextStatusLocked,
            ),
            (
                WorkspaceError::LinkedRepositoryIsReadOnly {
                    repository: repository_id(REPOSITORY_NAME),
                },
                WorkspaceErrorKind::LinkedRepositoryIsReadOnly,
            ),
        ];
        for (error, kind) in cases {
            assert_eq!(kind_of(WorkspaceServiceError::Rejected(error)), kind);
        }
    }

    #[test]
    fn every_service_failure_has_its_kind() {
        let storage = StorageError::Unavailable {
            reason: STORAGE_REASON.into(),
        };
        let cases = [
            (
                WorkspaceServiceError::ProjectNotFound {
                    tag: ProjectTag::parse(PROJECT_TAG).unwrap(),
                },
                WorkspaceErrorKind::ProjectNotFound,
            ),
            (
                WorkspaceServiceError::ProjectAlreadyExists {
                    tag: ProjectTag::parse(PROJECT_TAG).unwrap(),
                },
                WorkspaceErrorKind::ProjectAlreadyExists,
            ),
            (
                WorkspaceServiceError::Storage(storage),
                WorkspaceErrorKind::Storage,
            ),
            (
                WorkspaceServiceError::NewBranchAlreadyExists {
                    repository: REPOSITORY_NAME.into(),
                    branch: branch(),
                },
                WorkspaceErrorKind::BranchAlreadyExists,
            ),
            (
                git_failure(GitError::BranchAlreadyExists {
                    name: BRANCH.into(),
                }),
                WorkspaceErrorKind::BranchAlreadyExists,
            ),
            (
                WorkspaceServiceError::BranchCheckedOut {
                    repository: REPOSITORY_NAME.into(),
                    branch: branch(),
                    checked_out_in: CheckoutHolder::Folder(PROJECT_PATH.into()),
                },
                WorkspaceErrorKind::BranchCheckedOut,
            ),
            (
                git_failure(GitError::BranchCheckedOut {
                    name: BRANCH.into(),
                    worktree: PROJECT_PATH.into(),
                }),
                WorkspaceErrorKind::BranchCheckedOut,
            ),
            (git_failure(GitError::NoRemote), WorkspaceErrorKind::Git),
            (
                WorkspaceServiceError::Files(CheckoutFilesError {
                    path: PROJECT_PATH.into(),
                    reason: STORAGE_REASON.into(),
                }),
                WorkspaceErrorKind::Files,
            ),
        ];
        for (error, kind) in cases {
            assert_eq!(kind_of(error), kind);
        }
    }

    #[test]
    fn a_failed_update_keeps_its_cause() {
        let tag = ProjectTag::parse(PROJECT_TAG).unwrap();
        let cases = [
            (
                UpdateError::NotFound { tag },
                WorkspaceErrorKind::ProjectNotFound,
            ),
            (
                UpdateError::Rejected(WorkspaceError::DefaultContextIsFixed),
                WorkspaceErrorKind::DefaultContextIsFixed,
            ),
            (
                UpdateError::Storage(StorageError::Unavailable {
                    reason: STORAGE_REASON.into(),
                }),
                WorkspaceErrorKind::Storage,
            ),
        ];
        for (error, kind) in cases {
            assert_eq!(kind_of(WorkspaceServiceError::from(error)), kind);
        }
    }

    #[test]
    fn a_failed_task_is_internal() {
        assert_eq!(
            WorkspaceCommandError::task_failed("panicked").kind,
            WorkspaceErrorKind::Internal
        );
    }

    #[test]
    fn an_agent_runner_failure_is_agent_run() {
        use crate::workspace::application::error::AgentRunServiceError;
        use crate::workspace::domain::agent_runner::AgentRunError;

        let error = AgentRunServiceError::Runner(AgentRunError::UnknownTool {
            tool_id: "kiro".into(),
        });

        assert_eq!(kind_of(error), WorkspaceErrorKind::AgentRun);
    }

    #[test]
    fn no_running_agent_has_its_own_kind() {
        use crate::workspace::application::error::AgentRunServiceError;

        let error = AgentRunServiceError::NoRunningAgent {
            context: context_id(),
        };

        assert_eq!(kind_of(error), WorkspaceErrorKind::NoRunningAgent);
    }

    #[test]
    fn an_agent_run_workspace_failure_keeps_the_workspace_kind() {
        use crate::workspace::application::error::AgentRunServiceError;

        let error = AgentRunServiceError::Workspace(WorkspaceServiceError::ProjectNotFound {
            tag: ProjectTag::parse(PROJECT_TAG).unwrap(),
        });

        assert_eq!(kind_of(error), WorkspaceErrorKind::ProjectNotFound);
    }

    #[test]
    fn keeps_the_readable_message_and_serializes_the_kind_in_camel_case() {
        let error = WorkspaceCommandError::from(WorkspaceServiceError::BranchCheckedOut {
            repository: REPOSITORY_NAME.into(),
            branch: branch(),
            checked_out_in: CheckoutHolder::Context(ContextName::parse(CONTEXT_NAME).unwrap()),
        });

        let json = serde_json::to_value(&error).unwrap();

        assert_eq!(json["kind"], "branchCheckedOut");
        assert_eq!(
            json["message"],
            "Branch 'feature/login' of repository 'api' is already checked out in context 'Login page'"
        );
    }
}

mod create_context_input {
    use super::*;

    fn input(branches: Vec<RepositoryBranchInput>) -> CreateContextInput {
        CreateContextInput {
            project_tag: PROJECT_TAG.into(),
            name: CONTEXT_NAME.into(),
            base_context_id: None,
            origin: ContextOriginView::FromTask {
                connection_id: CONNECTION_ID.into(),
                task_id: TASK_ID.into(),
            },
            status: Some(ContextStatusView::InProgress),
            agent_prompt: None,
            branches,
        }
    }

    #[test]
    fn reads_the_wire_shape() {
        let json = serde_json::json!({
            "projectTag": PROJECT_TAG,
            "name": CONTEXT_NAME,
            "baseContextId": null,
            "origin": { "kind": "fromTask", "connectionId": CONNECTION_ID, "taskId": TASK_ID },
            "status": "inProgress",
            "agentPrompt": null,
            "branches": [{
                "repositoryId": REPOSITORY_NAME,
                "branch": { "kind": "newBranch", "name": BRANCH, "baseBranch": "main" },
            }],
        });

        let parsed: CreateContextInput = serde_json::from_value(json).unwrap();

        assert_eq!(
            parsed,
            input(vec![RepositoryBranchInput {
                repository_id: REPOSITORY_NAME.into(),
                branch: BranchChoiceInput::NewBranch {
                    name: BRANCH.into(),
                    base_branch: "main".into(),
                },
            }])
        );
    }

    #[test]
    fn becomes_the_create_command() {
        let command = CreateContextCommand::try_from(input(vec![RepositoryBranchInput {
            repository_id: REPOSITORY_NAME.into(),
            branch: BranchChoiceInput::ExistingBranch {
                name: BRANCH.into(),
            },
        }]))
        .unwrap();

        assert_eq!(command.status, Some(ContextStatus::InProgress));
        assert_eq!(
            command.origin,
            ContextOrigin::FromTask(TaskRef {
                connection_id: CONNECTION_ID.into(),
                task_id: TASK_ID.into(),
            })
        );
        assert_eq!(
            command.branches,
            [RepositoryBranchChoice {
                repository: repository_id(REPOSITORY_NAME),
                choice: BranchChoice::ExistingBranch { name: branch() },
            }]
        );
    }

    #[test]
    fn refuses_an_invalid_branch_name() {
        let result = CreateContextCommand::try_from(input(vec![RepositoryBranchInput {
            repository_id: REPOSITORY_NAME.into(),
            branch: BranchChoiceInput::ExistingBranch {
                name: "a..b".into(),
            },
        }]));

        assert_eq!(
            result.map_err(kind_of),
            Err(WorkspaceErrorKind::InvalidInput)
        );
    }
}

mod project_settings_input {
    use super::*;

    const PROJECT_NAME: &str = "Acme";
    const PREFIX: &str = "feature";
    const INVALID_PREFIX: &str = "my prefix";
    const INSTALL_COMMAND: &str = "pnpm install";
    const LINKED_AT: &str = "2024-01-15T10:00:00.000Z";

    fn wire_settings(prefixes: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "name": PROJECT_NAME,
            "path": PROJECT_PATH,
            "icon": null,
            "repositories": [{
                "id": REPOSITORY_NAME,
                "name": REPOSITORY_NAME,
                "relativePath": "./api",
                "postCheckoutCommand": INSTALL_COMMAND,
            }],
            "symlinks": [".env"],
            "branchNaming": { "prefixes": prefixes, "allowNoPrefix": false },
            "remoteProjectLinks": [{
                "providerKind": "teamwork",
                "connectionId": CONNECTION_ID,
                "remoteProjectId": TASK_ID,
                "remoteProjectName": PROJECT_NAME,
                "remoteProjectUrl": null,
                "linkedAt": LINKED_AT,
            }],
        })
    }

    fn convert(prefixes: &[&str]) -> Result<ProjectSettings, WorkspaceError> {
        let input: ProjectSettingsInput = serde_json::from_value(wire_settings(prefixes)).unwrap();
        ProjectSettings::try_from(input)
    }

    #[test]
    fn becomes_the_project_settings() {
        let settings = convert(&[PREFIX]).unwrap();

        assert_eq!(
            settings.repositories[0].post_checkout_command.as_deref(),
            Some(INSTALL_COMMAND)
        );
        assert_eq!(settings.remote_project_links[0].linked_at, LINKED_AT);
        assert_eq!(
            settings.branch_naming.default_prefix().unwrap().as_str(),
            PREFIX
        );
    }

    #[test]
    fn refuses_an_invalid_prefix_as_invalid_input() {
        assert_eq!(
            convert(&[PREFIX, INVALID_PREFIX]).map_err(kind_of),
            Err(WorkspaceErrorKind::InvalidInput)
        );
    }

    #[test]
    fn refuses_a_required_prefix_without_any_prefix_as_invalid_input() {
        assert_eq!(
            convert(&[]).map_err(kind_of),
            Err(WorkspaceErrorKind::InvalidInput)
        );
    }
}

mod project_view {
    use super::*;

    const API_ID: &str = "repo-1";

    fn project() -> Project {
        let api = repository(API_ID, REPOSITORY_NAME, "./api");
        let feature = Context {
            branches: vec![branch_on(API_ID, BRANCH)],
            ..context_named(CONTEXT_NAME)
        };
        project_with(vec![api])
            .repair_default_context(&HashMap::from([(
                repository_id(API_ID),
                vec![branch_name("main")],
            )]))
            .unwrap()
            .add_context(feature)
            .unwrap()
    }

    fn feature_view(view: &ProjectView) -> &ContextView {
        view.contexts
            .iter()
            .find(|context| context.name == CONTEXT_NAME)
            .unwrap()
    }

    #[test]
    fn carries_the_ready_made_paths() {
        let view = ProjectView::from(&project());

        let feature = feature_view(&view);
        assert_eq!(
            feature.folder_path,
            format!("{PROJECT_PATH}/.worktrees/login-page")
        );
        assert_eq!(
            feature.ide_path,
            format!("{PROJECT_PATH}/.worktrees/login-page/api")
        );
        assert_eq!(
            feature.branches[0].worktree_path,
            Some(format!("{PROJECT_PATH}/.worktrees/login-page/api"))
        );
        assert_eq!(
            view.effective_repositories[0].path,
            format!("{PROJECT_PATH}/api")
        );
    }

    #[test]
    fn describes_the_default_context_and_its_branches() {
        let view = ProjectView::from(&project());

        let default_context = view
            .contexts
            .iter()
            .find(|context| context.is_default)
            .unwrap();
        assert!(default_context.is_static);
        assert_eq!(default_context.status, ContextStatusView::Idle);
        assert_eq!(
            default_context.branches[0].worktree_path,
            Some(format!("{PROJECT_PATH}/api"))
        );
    }

    #[test]
    fn serializes_in_camel_case_with_nulls() {
        let json = serde_json::to_value(ProjectView::from(&project())).unwrap();

        assert_eq!(json["icon"], serde_json::Value::Null);
        assert_eq!(
            json["contexts"][1]["origin"],
            serde_json::json!({ "kind": "manual" })
        );
        assert_eq!(
            json["contexts"][1]["baseContextId"],
            serde_json::Value::Null
        );
        assert_eq!(json["branchNaming"]["allowsNoPrefix"], true);
    }
}

mod agent_run_view {
    use super::*;
    use crate::workspace::domain::agent_run::{AgentRun, AgentRunState};
    use crate::workspace::domain::identifiers::AgentRunId;

    const RUN_PROMPT: &str = "Build the login page";
    const STARTED_AT: &str = "2024-01-15T10:00:00Z";
    const OUTPUT_LINE: &str = "compiling";

    fn finished_run() -> AgentRun {
        let mut run = AgentRun::started(
            AgentRunId::parse("run-1").unwrap(),
            context_id(),
            RUN_PROMPT.to_string(),
            STARTED_AT.to_string(),
        );
        run.append_lines([OUTPUT_LINE.to_string()]);
        run.end(AgentRunState::Finished, Some(0));
        run
    }

    #[test]
    fn carries_the_transcript_state_and_start_time() {
        let view = AgentRunView::from(finished_run());

        assert_eq!(view.id, "run-1");
        assert_eq!(view.prompt, RUN_PROMPT);
        assert_eq!(view.output_lines, vec![OUTPUT_LINE]);
        assert_eq!(view.state, AgentRunStateView::Finished);
        assert_eq!(view.exit_code, Some(0));
        assert!(!view.trimmed);
        assert_eq!(view.started_at, STARTED_AT);
    }

    #[test]
    fn serializes_in_camel_case() {
        let json = serde_json::to_value(AgentRunView::from(finished_run())).unwrap();

        assert_eq!(json["id"], "run-1");
        assert_eq!(json["contextId"], context_id().to_string());
        assert_eq!(json["outputLines"], serde_json::json!([OUTPUT_LINE]));
        assert_eq!(json["state"], "finished");
        assert_eq!(json["exitCode"], 0);
        assert_eq!(json["startedAt"], STARTED_AT);
    }
}
