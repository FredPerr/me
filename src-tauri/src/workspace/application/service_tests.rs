//! Service tests on the in-memory project repository, the in-memory git fake
//! and the recording files and events fakes.
use std::path::PathBuf;

use super::*;
use crate::git::domain::remote_url::RemoteUrl;
use crate::git::infra::in_memory::{InMemoryGitRepositories, Operation};
use crate::workspace::application::error::CheckoutHolder;
use crate::workspace::domain::agent_work::AgentTool;
use crate::workspace::domain::branch_naming::BranchNamingPolicy;
use crate::workspace::domain::context_branch::ContextCheckout;
use crate::workspace::domain::context_branch::MatchedPullRequest;
use crate::workspace::domain::error::WorkspaceError;
use crate::workspace::domain::project::ProjectSettings;
use crate::workspace::domain::remote_project_link::RemoteProjectLink;
use crate::workspace::domain::test_support::{branch_name, repository, repository_id};
use crate::workspace::infra::in_memory_agent_run_repository::InMemoryAgentRunRepository;
use crate::workspace::infra::in_memory_project_repository::InMemoryProjectRepository;
use crate::workspace::infra::recording_fakes::{
    PostCheckoutRun, RecordingCheckoutFiles, RecordingProjectEvents,
};

const PROJECT_TAG: &str = "acme";
const PROJECT_NAME: &str = "Acme";
const PROJECT_PATH: &str = "/work/acme";
const API_ID: &str = "api-id";
const API_NAME: &str = "api";
const WEB_ID: &str = "web-id";
const WEB_NAME: &str = "web";
const DOCS_ID: &str = "docs-id";
const MAIN: &str = "main";
const LOGIN_CONTEXT: &str = "Login page";
const LOGIN_FOLDER_TWIN: &str = "Login-Page";
const OTHER_CONTEXT: &str = "Signup page";
const LOGIN_BRANCH: &str = "feature/login";
const SIGNUP_BRANCH: &str = "feature/signup";
const EXISTING_BRANCH: &str = "existing/web-work";
const TAKEN_BRANCH: &str = "feature/taken";
const INSTALL_COMMAND: &str = "pnpm install";
const ENV_SYMLINK: &str = ".env";
const PROMPT: &str = "Build the login page";
const LOGIN_API_WORKTREE: &str = "/work/acme/.worktrees/login-page/api";
const LOGIN_WEB_WORKTREE: &str = "/work/acme/.worktrees/login-page/web";
const LOGIN_FOLDER: &str = "/work/acme/.worktrees/login-page";
const LOGIN_ENV_LINK: &str = "/work/acme/.worktrees/login-page/.env";
const ORIGIN_URL: &str = "git@github.com:acme/api.git";

fn injected_failure() -> GitError {
    GitError::Io {
        action: "write".into(),
        detail: "disk full".into(),
    }
}

fn path_of(repository_name: &str) -> RepositoryPath {
    RepositoryPath::new(format!("{PROJECT_PATH}/{repository_name}"))
}

fn tag() -> ProjectTag {
    ProjectTag::parse(PROJECT_TAG).unwrap()
}

fn api() -> Repository {
    repository(API_ID, API_NAME, "./api")
}

fn web() -> Repository {
    Repository {
        post_checkout_command: Some(INSTALL_COMMAND.to_string()),
        ..repository(WEB_ID, WEB_NAME, "./web")
    }
}

fn project_settings(repositories: Vec<Repository>) -> ProjectSettings {
    ProjectSettings {
        name: PROJECT_NAME.to_string(),
        path: PROJECT_PATH.to_string(),
        icon: None,
        repositories,
        symlinks: vec![ENV_SYMLINK.to_string()],
        remote_project_links: vec![],
        branch_naming: BranchNamingPolicy::default(),
    }
}

fn on_main(repository_ids: &[&str]) -> HashMap<RepositoryId, Vec<BranchName>> {
    repository_ids
        .iter()
        .map(|id| (repository_id(id), vec![branch_name(MAIN)]))
        .collect()
}

fn new_branch(repository: &str, name: &str) -> RepositoryBranchChoice {
    RepositoryBranchChoice {
        repository: repository_id(repository),
        choice: BranchChoice::NewBranch {
            name: branch_name(name),
            base_branch: branch_name(MAIN),
        },
    }
}

fn existing_branch(repository: &str, name: &str) -> RepositoryBranchChoice {
    RepositoryBranchChoice {
        repository: repository_id(repository),
        choice: BranchChoice::ExistingBranch {
            name: branch_name(name),
        },
    }
}

fn create_command(name: &str, branches: Vec<RepositoryBranchChoice>) -> CreateContextCommand {
    CreateContextCommand {
        project: tag(),
        name: ContextName::parse(name).unwrap(),
        base: None,
        origin: ContextOrigin::Manual,
        status: None,
        agent_prompt: None,
        branches,
    }
}

/// The login context: a new branch in api, an existing branch in web.
fn login_command() -> CreateContextCommand {
    create_command(
        LOGIN_CONTEXT,
        vec![
            new_branch(API_ID, LOGIN_BRANCH),
            existing_branch(WEB_ID, EXISTING_BRANCH),
        ],
    )
}

/// The `acme` project with api and web on `main`, its default context on
/// both, and `existing/web-work` in web.
struct Fixture {
    projects: Arc<InMemoryProjectRepository>,
    git: Arc<InMemoryGitRepositories>,
    files: Arc<RecordingCheckoutFiles>,
    events: Arc<RecordingProjectEvents>,
    agent_runs: Arc<InMemoryAgentRunRepository>,
    service: WorkspaceService,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self::without_project();
        let project = Project::new(tag(), project_settings(vec![api(), web()]))
            .repair_default_context(&on_main(&[API_ID, WEB_ID]))
            .unwrap();
        fixture.projects.save(&project).unwrap();
        fixture
            .open(WEB_NAME)
            .create_branch(&branch_name(EXISTING_BRANCH), &branch_name(MAIN))
            .unwrap();
        fixture
    }

    fn without_project() -> Self {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let git = Arc::new(InMemoryGitRepositories::new());
        let files = Arc::new(RecordingCheckoutFiles::default());
        let events = Arc::new(RecordingProjectEvents::default());
        let agent_runs = Arc::new(InMemoryAgentRunRepository::default());
        git.add_repository(&path_of(API_NAME), &branch_name(MAIN));
        git.add_repository(&path_of(WEB_NAME), &branch_name(MAIN));
        let service = WorkspaceService::new(
            projects.clone(),
            git.clone(),
            files.clone(),
            events.clone(),
            agent_runs.clone(),
        );
        Self {
            projects,
            git,
            files,
            events,
            agent_runs,
            service,
        }
    }

    fn open(&self, repository_name: &str) -> Box<dyn GitRepository> {
        self.git.open(&path_of(repository_name)).unwrap()
    }

    fn has_branch(&self, repository_name: &str, branch: &str) -> bool {
        self.open(repository_name).has_branch(&branch_name(branch))
    }

    fn checkout_of(&self, repository_name: &str, branch: &str) -> Option<String> {
        self.open(repository_name)
            .worktree_for(&branch_name(branch))
            .unwrap()
            .map(|path| path.display())
    }

    fn linked_worktree_count(&self, repository_name: &str) -> usize {
        self.open(repository_name)
            .worktrees()
            .unwrap()
            .iter()
            .filter(|checkout| !checkout.is_main)
            .count()
    }

    fn stored(&self) -> Project {
        self.projects.find(&tag()).unwrap().unwrap()
    }

    fn stored_context(&self, name: &str) -> Option<Context> {
        self.stored()
            .contexts()
            .iter()
            .find(|context| context.name.as_str() == name)
            .cloned()
    }

    fn create_login(&self) -> ContextId {
        let project = self.service.create_context(login_command()).unwrap();
        context_id(&project, LOGIN_CONTEXT)
    }

    fn change_count(&self) -> usize {
        self.events.changed_projects().len()
    }
}

fn context_id(project: &Project, name: &str) -> ContextId {
    project
        .contexts()
        .iter()
        .find(|context| context.name.as_str() == name)
        .map(|context| context.id.clone())
        .unwrap()
}

fn branches_of(context: &Context) -> Vec<(String, String)> {
    context
        .branches
        .iter()
        .map(|branch| (branch.repository.to_string(), branch.branch.to_string()))
        .collect()
}

mod create_context {
    use super::*;

    #[test]
    fn saves_the_context_with_a_branch_per_repository() {
        let fixture = Fixture::new();

        fixture.create_login();

        let context = fixture.stored_context(LOGIN_CONTEXT).unwrap();
        assert_eq!(
            branches_of(&context),
            [
                (API_ID.to_string(), LOGIN_BRANCH.to_string()),
                (WEB_ID.to_string(), EXISTING_BRANCH.to_string()),
            ]
        );
    }

    #[test]
    fn creates_new_branches_and_checks_every_branch_out_where_the_domain_says() {
        let fixture = Fixture::new();

        fixture.create_login();

        let project = fixture.stored();
        let context = fixture.stored_context(LOGIN_CONTEXT).unwrap();
        assert_eq!(
            fixture.checkout_of(API_NAME, LOGIN_BRANCH),
            Some(project.worktree_path(&context, &api()))
        );
        assert_eq!(
            fixture.checkout_of(WEB_NAME, EXISTING_BRANCH).as_deref(),
            Some(LOGIN_WEB_WORKTREE)
        );
    }

    #[test]
    fn keeps_the_origin_status_base_and_prompt() {
        let fixture = Fixture::new();
        let default_id = fixture.stored().default_context().unwrap().id.clone();
        let command = CreateContextCommand {
            base: Some(default_id.clone()),
            origin: ContextOrigin::FromBranches,
            status: Some(ContextStatus::InProgress),
            agent_prompt: Some(PROMPT.to_string()),
            ..login_command()
        };

        fixture.service.create_context(command).unwrap();

        let context = fixture.stored_context(LOGIN_CONTEXT).unwrap();
        assert_eq!(context.base, Some(default_id));
        assert_eq!(context.origin, ContextOrigin::FromBranches);
        assert_eq!(context.status, ContextStatus::InProgress);
        assert_eq!(context.agent.unwrap().prompt.as_deref(), Some(PROMPT));
    }

    #[test]
    fn returns_the_saved_project_and_announces_the_change() {
        let fixture = Fixture::new();

        let project = fixture.service.create_context(login_command()).unwrap();

        assert_eq!(project, fixture.stored());
        assert_eq!(fixture.events.changed_projects(), [tag()]);
    }

    #[test]
    fn runs_post_checkout_commands_and_links_symlinks_once_checked_out() {
        let fixture = Fixture::new();

        fixture.create_login();

        assert_eq!(
            fixture.files.post_checkout_runs(),
            [PostCheckoutRun {
                command: INSTALL_COMMAND.to_string(),
                checkout: PathBuf::from(LOGIN_WEB_WORKTREE),
            }]
        );
        assert_eq!(fixture.files.symlinks(), [PathBuf::from(LOGIN_ENV_LINK)]);
    }

    /// Opening a repository fails in these tests, so any git work would
    /// report that failure instead of the refusal.
    #[test]
    fn refuses_a_duplicate_name_before_any_git_work() {
        let fixture = Fixture::new();
        fixture.create_login();
        fixture.git.fail_on(Operation::Open, injected_failure());
        let command = create_command(
            &LOGIN_CONTEXT.to_uppercase(),
            vec![new_branch(API_ID, SIGNUP_BRANCH)],
        );

        let result = fixture.service.create_context(command);

        assert_eq!(
            result,
            Err(WorkspaceServiceError::Rejected(
                WorkspaceError::DuplicateContextName {
                    name: LOGIN_CONTEXT.to_uppercase()
                }
            ))
        );
    }

    #[test]
    fn refuses_a_duplicate_folder_before_any_git_work() {
        let fixture = Fixture::new();
        fixture.create_login();
        fixture.git.fail_on(Operation::Open, injected_failure());
        let command = create_command(LOGIN_FOLDER_TWIN, vec![new_branch(API_ID, SIGNUP_BRANCH)]);

        let result = fixture.service.create_context(command);

        assert!(matches!(
            result,
            Err(WorkspaceServiceError::Rejected(
                WorkspaceError::DuplicateContextFolder { .. }
            ))
        ));
    }

    #[test]
    fn refuses_a_new_branch_name_that_already_exists() {
        let fixture = Fixture::new();
        fixture
            .open(API_NAME)
            .create_branch(&branch_name(TAKEN_BRANCH), &branch_name(MAIN))
            .unwrap();
        let command = create_command(LOGIN_CONTEXT, vec![new_branch(API_ID, TAKEN_BRANCH)]);

        let result = fixture.service.create_context(command);

        assert_eq!(
            result,
            Err(WorkspaceServiceError::NewBranchAlreadyExists {
                repository: API_NAME.to_string(),
                branch: branch_name(TAKEN_BRANCH),
            })
        );
        assert_eq!(fixture.linked_worktree_count(API_NAME), 0);
        assert!(fixture.stored_context(LOGIN_CONTEXT).is_none());
    }

    #[test]
    fn refuses_a_branch_checked_out_in_another_context_naming_it() {
        let fixture = Fixture::new();
        fixture.create_login();
        let command = create_command(
            OTHER_CONTEXT,
            vec![existing_branch(WEB_ID, EXISTING_BRANCH)],
        );

        let error = fixture.service.create_context(command).unwrap_err();

        assert_eq!(
            error,
            WorkspaceServiceError::BranchCheckedOut {
                repository: WEB_NAME.to_string(),
                branch: branch_name(EXISTING_BRANCH),
                checked_out_in: CheckoutHolder::Context(ContextName::parse(LOGIN_CONTEXT).unwrap()),
            }
        );
        assert!(error.to_string().contains(LOGIN_CONTEXT));
    }

    #[test]
    fn names_the_default_context_for_a_branch_of_the_main_checkout() {
        let fixture = Fixture::new();
        let command = create_command(OTHER_CONTEXT, vec![existing_branch(API_ID, MAIN)]);

        let error = fixture.service.create_context(command).unwrap_err();

        assert!(matches!(
            error,
            WorkspaceServiceError::BranchCheckedOut {
                checked_out_in: CheckoutHolder::Context(name),
                ..
            } if name.as_str() == "default"
        ));
    }

    mod when_a_later_repository_fails {
        use super::*;

        fn fixture_failing_in_web(operation: Operation) -> Fixture {
            let fixture = Fixture::new();
            fixture
                .git
                .fail_on_in(&path_of(WEB_NAME), operation, injected_failure());
            fixture
        }

        fn create_on_both(fixture: &Fixture) -> Result<Project, WorkspaceServiceError> {
            fixture.service.create_context(create_command(
                LOGIN_CONTEXT,
                vec![
                    new_branch(API_ID, LOGIN_BRANCH),
                    new_branch(WEB_ID, LOGIN_BRANCH),
                ],
            ))
        }

        #[test]
        fn reports_the_failure_with_the_repository() {
            let fixture = fixture_failing_in_web(Operation::AddWorktree);

            assert_eq!(
                create_on_both(&fixture),
                Err(WorkspaceServiceError::git(WEB_NAME, injected_failure()))
            );
        }

        #[test]
        fn removes_the_worktrees_and_branches_already_made() {
            let fixture = fixture_failing_in_web(Operation::AddWorktree);

            let _ = create_on_both(&fixture);

            assert_eq!(fixture.linked_worktree_count(API_NAME), 0);
            assert!(!fixture.has_branch(API_NAME, LOGIN_BRANCH));
            assert!(!fixture.has_branch(WEB_NAME, LOGIN_BRANCH));
        }

        #[test]
        fn saves_nothing_and_announces_nothing() {
            let fixture = fixture_failing_in_web(Operation::AddWorktree);

            let _ = create_on_both(&fixture);

            assert!(fixture.stored_context(LOGIN_CONTEXT).is_none());
            assert_eq!(fixture.change_count(), 0);
        }

        #[test]
        fn runs_no_post_checkout_command_and_links_nothing() {
            let fixture = fixture_failing_in_web(Operation::AddWorktree);

            let _ = create_on_both(&fixture);

            assert!(fixture.files.post_checkout_runs().is_empty());
            assert!(fixture.files.symlinks().is_empty());
        }

        #[test]
        fn removes_the_context_folder_once_empty() {
            let fixture = fixture_failing_in_web(Operation::CreateBranch);

            let _ = create_on_both(&fixture);

            assert_eq!(
                fixture.files.removed_folders(),
                [PathBuf::from(LOGIN_FOLDER)]
            );
        }
    }

    #[test]
    fn a_failing_symlink_undoes_every_checkout() {
        let fixture = Fixture::new();
        fixture.files.fail_on_link(LOGIN_ENV_LINK);

        let result = fixture.service.create_context(login_command());

        assert!(matches!(result, Err(WorkspaceServiceError::Files(_))));
        assert_eq!(fixture.linked_worktree_count(API_NAME), 0);
        assert_eq!(fixture.linked_worktree_count(WEB_NAME), 0);
        assert!(!fixture.has_branch(API_NAME, LOGIN_BRANCH));
        assert!(fixture.has_branch(WEB_NAME, EXISTING_BRANCH));
        assert!(fixture.stored_context(LOGIN_CONTEXT).is_none());
    }

    mod from_pull_requests {
        use super::*;

        const PR_HOST: &str = "github";
        const PR_NUMBER: u32 = 12;
        const PR_URL: &str = "https://github.com/acme/api/pull/12";
        const PR_HEAD: &str = "feature/login";
        const PR_BASE: &str = "develop";
        const DOCS_API_ORIGIN: &str = "git@github.com:acme/api.git";

        fn matched() -> MatchedPullRequest {
            MatchedPullRequest {
                host: PR_HOST.to_string(),
                number: PR_NUMBER,
                url: PR_URL.to_string(),
            }
        }

        fn pull_request_branch(repository: &str) -> RepositoryBranchChoice {
            RepositoryBranchChoice {
                repository: repository_id(repository),
                choice: BranchChoice::PullRequest {
                    head: branch_name(PR_HEAD),
                    base: branch_name(PR_BASE),
                    matched: matched(),
                },
            }
        }

        /// Puts `PR_HEAD` and `PR_BASE` on api's `origin` only, so a fetch has
        /// to bring them down. `api` is left with just `main` locally.
        fn api_with_pull_request_on_origin(fixture: &Fixture) {
            let api = fixture.open(API_NAME);
            fixture
                .git
                .set_remote(&path_of(API_NAME), RemoteUrl::new(DOCS_API_ORIGIN));
            for branch in [PR_HEAD, PR_BASE] {
                api.create_branch(&branch_name(branch), &branch_name(MAIN))
                    .unwrap();
                fixture
                    .git
                    .publish(&path_of(API_NAME), &branch_name(branch));
                api.delete_branch(&branch_name(branch), true).unwrap();
            }
        }

        fn create_from_pull_request(
            fixture: &Fixture,
            branches: Vec<RepositoryBranchChoice>,
        ) -> Result<Project, WorkspaceServiceError> {
            fixture.service.create_context(CreateContextCommand {
                origin: ContextOrigin::FromPullRequests,
                status: Some(ContextStatus::Review),
                ..create_command(LOGIN_CONTEXT, branches)
            })
        }

        #[test]
        fn fetches_the_head_and_base_before_checking_out() {
            let fixture = Fixture::new();
            api_with_pull_request_on_origin(&fixture);

            create_from_pull_request(&fixture, vec![pull_request_branch(API_ID)]).unwrap();

            assert_eq!(
                fixture.git.fetched_branches(&path_of(API_NAME)),
                [branch_name(PR_HEAD), branch_name(PR_BASE)]
            );
            assert!(fixture.has_branch(API_NAME, PR_HEAD));
            assert!(fixture.has_branch(API_NAME, PR_BASE));
        }

        #[test]
        fn records_the_matched_pull_request_on_the_branch() {
            let fixture = Fixture::new();
            api_with_pull_request_on_origin(&fixture);

            create_from_pull_request(&fixture, vec![pull_request_branch(API_ID)]).unwrap();

            let branch = fixture
                .stored_context(LOGIN_CONTEXT)
                .unwrap()
                .branch_for(&repository_id(API_ID))
                .unwrap()
                .clone();
            assert_eq!(branch.branch, branch_name(PR_HEAD));
            let link = branch.pull_request.unwrap();
            assert_eq!(link.matched, Some(matched()));
            assert_eq!(link.draft, None);
        }

        #[test]
        fn starts_in_review_from_pull_requests() {
            let fixture = Fixture::new();
            api_with_pull_request_on_origin(&fixture);

            create_from_pull_request(&fixture, vec![pull_request_branch(API_ID)]).unwrap();

            let context = fixture.stored_context(LOGIN_CONTEXT).unwrap();
            assert_eq!(context.origin, ContextOrigin::FromPullRequests);
            assert_eq!(context.status, ContextStatus::Review);
        }

        #[test]
        fn gives_a_repository_without_a_pull_request_its_new_branch_from_the_selected_base() {
            let fixture = Fixture::new();
            api_with_pull_request_on_origin(&fixture);
            fixture
                .open(WEB_NAME)
                .create_branch(&branch_name(PR_BASE), &branch_name(MAIN))
                .unwrap();

            create_from_pull_request(
                &fixture,
                vec![
                    pull_request_branch(API_ID),
                    RepositoryBranchChoice {
                        repository: repository_id(WEB_ID),
                        choice: BranchChoice::NewBranch {
                            name: branch_name(SIGNUP_BRANCH),
                            base_branch: branch_name(PR_BASE),
                        },
                    },
                ],
            )
            .unwrap();

            let web_branch = fixture
                .stored_context(LOGIN_CONTEXT)
                .unwrap()
                .branch_for(&repository_id(WEB_ID))
                .unwrap()
                .clone();
            assert_eq!(web_branch.branch, branch_name(SIGNUP_BRANCH));
            assert_eq!(web_branch.pull_request, None);
            assert!(fixture.has_branch(WEB_NAME, SIGNUP_BRANCH));
        }

        #[test]
        fn creates_nothing_when_a_fetch_fails_and_names_the_pull_request_and_branch() {
            let fixture = Fixture::new();
            api_with_pull_request_on_origin(&fixture);
            fixture.git.fail_on_in(
                &path_of(API_NAME),
                Operation::FetchBranch,
                injected_failure(),
            );

            let error =
                create_from_pull_request(&fixture, vec![pull_request_branch(API_ID)]).unwrap_err();

            assert_eq!(
                error,
                WorkspaceServiceError::PullRequestBranchNotFetched {
                    repository: API_NAME.to_string(),
                    pull_request_number: PR_NUMBER,
                    branch: branch_name(PR_HEAD),
                    error: injected_failure(),
                }
            );
            let message = error.to_string();
            assert!(message.contains(&format!("#{PR_NUMBER}")));
            assert!(message.contains(PR_HEAD));
            assert!(fixture.stored_context(LOGIN_CONTEXT).is_none());
            assert_eq!(fixture.linked_worktree_count(API_NAME), 0);
            assert_eq!(fixture.change_count(), 0);
        }
    }
}

mod delete_context {
    use super::*;

    #[test]
    fn removes_the_worktrees_then_the_context_and_announces_it() {
        let fixture = Fixture::new();
        let id = fixture.create_login();

        let project = fixture.service.delete_context(&tag(), &id).unwrap();

        assert_eq!(fixture.linked_worktree_count(API_NAME), 0);
        assert_eq!(fixture.linked_worktree_count(WEB_NAME), 0);
        assert!(project.find_context(&id).is_none());
        assert_eq!(project, fixture.stored());
        assert_eq!(fixture.change_count(), 2);
    }

    #[test]
    fn removes_the_configured_symlinks_and_the_empty_context_folder() {
        let fixture = Fixture::new();
        let id = fixture.create_login();

        fixture.service.delete_context(&tag(), &id).unwrap();

        assert!(fixture.files.symlinks().is_empty());
        assert_eq!(
            fixture.files.removed_folders(),
            [PathBuf::from(LOGIN_FOLDER)]
        );
    }

    #[test]
    fn deletes_branches_without_commits_beyond_their_base() {
        let fixture = Fixture::new();
        let id = fixture.create_login();

        fixture.service.delete_context(&tag(), &id).unwrap();

        assert!(!fixture.has_branch(API_NAME, LOGIN_BRANCH));
        assert!(!fixture.has_branch(WEB_NAME, EXISTING_BRANCH));
        assert!(fixture.has_branch(API_NAME, MAIN));
    }

    #[test]
    fn keeps_a_branch_with_commits() {
        let fixture = Fixture::new();
        let id = fixture.create_login();
        fixture
            .git
            .commit(&path_of(API_NAME), &branch_name(LOGIN_BRANCH));

        fixture.service.delete_context(&tag(), &id).unwrap();

        assert!(fixture.has_branch(API_NAME, LOGIN_BRANCH));
    }

    #[test]
    fn keeps_a_pushed_branch() {
        let fixture = Fixture::new();
        let id = fixture.create_login();
        fixture.git.set_remote(
            &path_of(API_NAME),
            crate::git::domain::remote_url::RemoteUrl::new(ORIGIN_URL),
        );
        fixture
            .git
            .publish(&path_of(API_NAME), &branch_name(LOGIN_BRANCH));

        fixture.service.delete_context(&tag(), &id).unwrap();

        assert!(fixture.has_branch(API_NAME, LOGIN_BRANCH));
    }

    #[test]
    fn measures_commits_from_the_base_contexts_branch() {
        let fixture = Fixture::new();
        let login_id = fixture.create_login();
        fixture
            .git
            .commit(&path_of(API_NAME), &branch_name(LOGIN_BRANCH));
        let signup = CreateContextCommand {
            base: Some(login_id),
            ..create_command(
                OTHER_CONTEXT,
                vec![RepositoryBranchChoice {
                    repository: repository_id(API_ID),
                    choice: BranchChoice::NewBranch {
                        name: branch_name(SIGNUP_BRANCH),
                        base_branch: branch_name(LOGIN_BRANCH),
                    },
                }],
            )
        };
        let project = fixture.service.create_context(signup).unwrap();

        fixture
            .service
            .delete_context(&tag(), &context_id(&project, OTHER_CONTEXT))
            .unwrap();

        assert!(!fixture.has_branch(API_NAME, SIGNUP_BRANCH));
    }

    #[test]
    fn refuses_the_default_context() {
        let fixture = Fixture::new();
        let default_id = fixture.stored().default_context().unwrap().id.clone();

        let result = fixture.service.delete_context(&tag(), &default_id);

        assert_eq!(
            result,
            Err(WorkspaceServiceError::Rejected(
                WorkspaceError::DefaultContextIsFixed
            ))
        );
    }

    #[test]
    fn deletes_the_contexts_agent_runs() {
        use crate::workspace::domain::agent_run::AgentRun;
        use crate::workspace::domain::identifiers::AgentRunId;
        use crate::workspace::domain::ports::AgentRunRepository;

        let fixture = Fixture::new();
        let id = fixture.create_login();
        let run = AgentRun::started(
            AgentRunId::generate(),
            id.clone(),
            PROMPT.to_string(),
            "2024-01-15T10:00:00Z".to_string(),
        );
        fixture.agent_runs.start_run(&run).unwrap();

        fixture.service.delete_context(&tag(), &id).unwrap();

        assert!(fixture.agent_runs.list_runs(&id).unwrap().is_empty());
    }

    #[test]
    fn saves_nothing_when_a_worktree_cant_be_removed() {
        let fixture = Fixture::new();
        let id = fixture.create_login();
        fixture.git.fail_on_in(
            &path_of(WEB_NAME),
            Operation::RemoveWorktree,
            injected_failure(),
        );

        let result = fixture.service.delete_context(&tag(), &id);

        assert_eq!(
            result,
            Err(WorkspaceServiceError::git(WEB_NAME, injected_failure()))
        );
        assert!(fixture.stored().find_context(&id).is_some());
        assert!(fixture.has_branch(API_NAME, LOGIN_BRANCH));
        assert_eq!(fixture.change_count(), 1);
    }

    #[test]
    fn goes_on_when_the_worktree_is_already_gone() {
        let fixture = Fixture::new();
        let id = fixture.create_login();
        fixture
            .open(API_NAME)
            .remove_worktree(&WorktreePath::new(LOGIN_API_WORKTREE))
            .unwrap();

        let project = fixture.service.delete_context(&tag(), &id).unwrap();

        assert!(project.find_context(&id).is_none());
    }

    #[test]
    fn removes_only_the_symlink_of_a_legacy_linked_repository() {
        let fixture = Fixture::new();
        let linked = Context {
            branches: vec![ContextBranch {
                checkout: ContextCheckout::Linked,
                ..ContextBranch::new(repository_id(WEB_ID), branch_name(MAIN))
            }],
            ..Context::new(ContextName::parse(LOGIN_CONTEXT).unwrap(), vec![]).unwrap()
        };
        let id = linked.id.clone();
        fixture
            .projects
            .save(&fixture.stored().add_legacy_context(linked).unwrap())
            .unwrap();
        fixture.files.add_symlink(LOGIN_WEB_WORKTREE);
        fixture
            .git
            .fail_on(Operation::RemoveWorktree, injected_failure());

        let project = fixture.service.delete_context(&tag(), &id).unwrap();

        assert!(project.find_context(&id).is_none());
        assert!(fixture.files.symlinks().is_empty());
        assert!(fixture.has_branch(WEB_NAME, MAIN));
    }
}

mod default_context_repair {
    use super::*;

    const DOCS_NAME: &str = "docs";

    fn save_project(fixture: &Fixture, project: &Project) {
        fixture.projects.save(project).unwrap();
    }

    fn default_branches(project: &Project) -> Vec<(String, String)> {
        branches_of(project.default_context().unwrap())
    }

    #[test]
    fn adds_the_branch_of_a_repository_the_default_context_lacks_and_saves_it() {
        let fixture = Fixture::without_project();
        let project = Project::new(tag(), project_settings(vec![api(), web()]))
            .repair_default_context(&on_main(&[API_ID]))
            .unwrap();
        save_project(&fixture, &project);

        let loaded = fixture.service.load_project(&tag()).unwrap();

        let expected = [
            (API_ID.to_string(), MAIN.to_string()),
            (WEB_ID.to_string(), MAIN.to_string()),
        ];
        assert_eq!(default_branches(&loaded), expected);
        assert_eq!(default_branches(&fixture.stored()), expected);
        assert_eq!(fixture.events.changed_projects(), [tag()]);
    }

    #[test]
    fn creates_a_missing_default_context_when_listing() {
        let fixture = Fixture::without_project();
        save_project(
            &fixture,
            &Project::new(tag(), project_settings(vec![api()])),
        );

        let listing = fixture.service.list_projects().unwrap();

        assert_eq!(
            default_branches(&listing.projects[0]),
            [(API_ID.to_string(), MAIN.to_string())]
        );
        assert!(fixture.stored().default_context().is_some());
    }

    #[test]
    fn skips_repositories_that_cant_be_opened() {
        let fixture = Fixture::without_project();
        let docs = repository(DOCS_ID, DOCS_NAME, "./docs");
        save_project(
            &fixture,
            &Project::new(tag(), project_settings(vec![api(), docs])),
        );

        let loaded = fixture.service.load_project(&tag()).unwrap();

        assert_eq!(
            default_branches(&loaded),
            [(API_ID.to_string(), MAIN.to_string())]
        );
    }

    #[test]
    fn saves_nothing_when_the_default_context_is_complete() {
        let fixture = Fixture::new();

        fixture.service.load_project(&tag()).unwrap();

        assert_eq!(fixture.change_count(), 0);
    }

    #[test]
    fn reports_an_unknown_project() {
        let fixture = Fixture::without_project();

        assert_eq!(
            fixture.service.load_project(&tag()),
            Err(WorkspaceServiceError::ProjectNotFound { tag: tag() })
        );
    }
}

mod context_changes {
    use super::*;

    const CONVERSATION_ID: &str = "kiro-conversation-1";
    const DRAFT_TITLE: &str = "Add the login page";
    const DRAFT_DESCRIPTION: &str = "Adds the login form";

    fn draft() -> PullRequestDraft {
        PullRequestDraft {
            title: DRAFT_TITLE.to_string(),
            description: DRAFT_DESCRIPTION.to_string(),
        }
    }

    fn create_signup(fixture: &Fixture) -> ContextId {
        let command = create_command(OTHER_CONTEXT, vec![new_branch(API_ID, SIGNUP_BRANCH)]);
        let project = fixture.service.create_context(command).unwrap();
        context_id(&project, OTHER_CONTEXT)
    }

    #[test]
    fn sets_the_status_saves_it_and_announces_it() {
        let fixture = Fixture::new();
        let id = fixture.create_login();

        let project = fixture
            .service
            .set_context_status(&tag(), &id, ContextStatus::Review)
            .unwrap();

        assert_eq!(project, fixture.stored());
        assert_eq!(
            fixture.stored().find_context(&id).unwrap().status,
            ContextStatus::Review
        );
        assert_eq!(fixture.change_count(), 2);
    }

    #[test]
    fn refuses_a_status_change_on_a_static_context() {
        let fixture = Fixture::new();
        let id = fixture.create_login();
        fixture
            .service
            .set_context_static(&tag(), &id, true)
            .unwrap();

        let result = fixture
            .service
            .set_context_status(&tag(), &id, ContextStatus::Done);

        assert_eq!(
            result,
            Err(WorkspaceServiceError::Rejected(
                WorkspaceError::StaticContextStatusLocked { id: id.clone() }
            ))
        );
        assert_eq!(
            fixture.stored().find_context(&id).unwrap().status,
            ContextStatus::Ready
        );
        assert_eq!(fixture.change_count(), 2);
    }

    #[test]
    fn marks_a_context_static_and_announces_it() {
        let fixture = Fixture::new();
        let id = fixture.create_login();

        fixture
            .service
            .set_context_static(&tag(), &id, true)
            .unwrap();

        assert!(fixture.stored().find_context(&id).unwrap().is_static);
        assert_eq!(fixture.change_count(), 2);
    }

    #[test]
    fn saves_a_pull_request_draft_on_the_context_branch() {
        let fixture = Fixture::new();
        let id = fixture.create_login();

        fixture
            .service
            .save_pull_request_draft(&tag(), &id, &repository_id(API_ID), draft())
            .unwrap();

        let stored = fixture.stored();
        let branch = stored
            .find_context(&id)
            .unwrap()
            .branch_for(&repository_id(API_ID))
            .unwrap();
        assert_eq!(branch.pull_request.as_ref().unwrap().draft, Some(draft()));
        assert_eq!(fixture.change_count(), 2);
    }

    #[test]
    fn sets_then_clears_the_agent_prompt() {
        let fixture = Fixture::new();
        let id = fixture.create_login();

        fixture
            .service
            .set_agent_prompt(&tag(), &id, Some(PROMPT.to_string()))
            .unwrap();
        let prompt_after_setting = fixture.stored_context(LOGIN_CONTEXT).unwrap().agent;
        fixture.service.set_agent_prompt(&tag(), &id, None).unwrap();

        assert_eq!(
            prompt_after_setting.unwrap().prompt.as_deref(),
            Some(PROMPT)
        );
        let cleared = fixture
            .stored_context(LOGIN_CONTEXT)
            .unwrap()
            .agent
            .unwrap();
        assert_eq!(cleared.prompt, None);
        assert_eq!(fixture.change_count(), 3);
    }

    #[test]
    fn records_the_kiro_conversation() {
        let fixture = Fixture::new();
        let id = fixture.create_login();

        fixture
            .service
            .record_agent_conversation(&tag(), &id, CONVERSATION_ID.to_string())
            .unwrap();

        let agent = fixture
            .stored_context(LOGIN_CONTEXT)
            .unwrap()
            .agent
            .unwrap();
        assert_eq!(
            agent.tool,
            AgentTool::Kiro {
                conversation_id: Some(CONVERSATION_ID.to_string())
            }
        );
        assert_eq!(fixture.change_count(), 2);
    }

    #[test]
    fn reports_an_unknown_project_and_announces_nothing() {
        let fixture = Fixture::without_project();
        let id = ContextId::generate();

        let result = fixture
            .service
            .set_context_status(&tag(), &id, ContextStatus::Review);

        assert_eq!(
            result,
            Err(WorkspaceServiceError::ProjectNotFound { tag: tag() })
        );
        assert_eq!(fixture.change_count(), 0);
    }

    #[test]
    fn keeps_both_of_two_concurrent_status_changes() {
        let fixture = Fixture::new();
        let login_id = fixture.create_login();
        let signup_id = create_signup(&fixture);

        std::thread::scope(|scope| {
            for id in [&login_id, &signup_id] {
                let service = &fixture.service;
                scope.spawn(move || {
                    service
                        .set_context_status(&tag(), id, ContextStatus::InProgress)
                        .unwrap()
                });
            }
        });

        let stored = fixture.stored();
        for id in [&login_id, &signup_id] {
            assert_eq!(
                stored.find_context(id).unwrap().status,
                ContextStatus::InProgress
            );
        }
    }
}

mod project_settings {
    use super::*;

    const RENAMED_PROJECT: &str = "Acme Platform";
    const ICON: &str = "rocket";
    const REQUIRED_PREFIX: &str = "ops";
    const NODE_MODULES_SYMLINK: &str = "node_modules";

    fn link() -> RemoteProjectLink {
        RemoteProjectLink {
            provider_kind: "teamwork".to_string(),
            connection_id: "teamwork:acme.teamwork.com".to_string(),
            remote_project_id: "42".to_string(),
            remote_project_name: "Website".to_string(),
            remote_project_url: None,
            linked_at: "2024-01-15T10:00:00.000Z".to_string(),
        }
    }

    fn changed_settings() -> ProjectSettings {
        ProjectSettings {
            name: RENAMED_PROJECT.to_string(),
            icon: Some(ICON.to_string()),
            symlinks: vec![NODE_MODULES_SYMLINK.to_string()],
            remote_project_links: vec![link()],
            branch_naming: BranchNamingPolicy::parse(&[REQUIRED_PREFIX], false).unwrap(),
            ..project_settings(vec![api()])
        }
    }

    #[test]
    fn saves_every_setting_in_one_change_and_announces_it() {
        let fixture = Fixture::new();

        let project = fixture
            .service
            .update_project_settings(&tag(), changed_settings())
            .unwrap();

        let stored = fixture.stored();
        assert_eq!(project, stored);
        assert_eq!(stored.name(), RENAMED_PROJECT);
        assert_eq!(stored.icon(), Some(ICON));
        assert_eq!(stored.repositories(), [api()]);
        assert_eq!(stored.symlinks(), [NODE_MODULES_SYMLINK]);
        assert_eq!(stored.remote_project_links(), [link()]);
        assert_eq!(
            stored.branch_naming().default_prefix().unwrap().as_str(),
            REQUIRED_PREFIX
        );
        assert_eq!(fixture.events.changed_projects(), [tag()]);
    }

    #[test]
    fn keeps_the_tag_and_the_contexts() {
        let fixture = Fixture::new();
        fixture.create_login();
        let contexts_before = fixture.stored().contexts().to_vec();

        fixture
            .service
            .update_project_settings(&tag(), changed_settings())
            .unwrap();

        let stored = fixture.stored();
        assert_eq!(stored.tag(), &tag());
        assert_eq!(stored.contexts(), contexts_before);
    }

    #[test]
    fn refuses_settings_the_project_rejects_and_saves_nothing() {
        let fixture = Fixture::new();
        let stored_before = fixture.stored();
        let twin = repository(API_ID, WEB_NAME, "./twin");

        let result = fixture
            .service
            .update_project_settings(&tag(), project_settings(vec![api(), twin]));

        assert_eq!(
            result,
            Err(WorkspaceServiceError::Rejected(
                WorkspaceError::DuplicateRepository {
                    id: repository_id(API_ID)
                }
            ))
        );
        assert_eq!(fixture.stored(), stored_before);
        assert_eq!(fixture.change_count(), 0);
    }

    #[test]
    fn an_invalid_prefix_never_becomes_settings() {
        assert!(matches!(
            BranchNamingPolicy::parse(&["my prefix"], true),
            Err(WorkspaceError::InvalidPrefix { .. })
        ));
    }
}

mod add_project {
    use super::*;

    const DEVELOP: &str = "develop";
    const MASTER: &str = "master";
    const TRUNK: &str = "trunk";
    const RELEASE: &str = "release";
    const LEGACY_ID: &str = "legacy-id";
    const LEGACY_NAME: &str = "legacy";
    const TOOLS_ID: &str = "tools-id";
    const TOOLS_NAME: &str = "tools";
    const DOCS_NAME: &str = "docs";

    fn add_repository(fixture: &Fixture, name: &str, default_branch: &str, others: &[&str]) {
        fixture
            .git
            .add_repository(&path_of(name), &branch_name(default_branch));
        for other in others {
            fixture
                .open(name)
                .create_branch(&branch_name(other), &branch_name(default_branch))
                .unwrap();
        }
    }

    fn default_branches(project: &Project) -> Vec<(String, String)> {
        branches_of(project.default_context().unwrap())
    }

    #[test]
    fn saves_the_project_and_announces_it() {
        let fixture = Fixture::without_project();

        let project = fixture
            .service
            .add_project(tag(), project_settings(vec![api(), web()]))
            .unwrap();

        assert_eq!(project, fixture.stored());
        assert_eq!(fixture.events.changed_projects(), [tag()]);
    }

    #[test]
    fn puts_the_default_context_on_main_before_master_or_the_first_branch() {
        let fixture = Fixture::without_project();
        fixture
            .open(API_NAME)
            .create_branch(&branch_name(DEVELOP), &branch_name(MAIN))
            .unwrap();
        fixture
            .open(API_NAME)
            .create_branch(&branch_name(MASTER), &branch_name(MAIN))
            .unwrap();
        add_repository(&fixture, LEGACY_NAME, MASTER, &[DEVELOP]);
        add_repository(&fixture, TOOLS_NAME, TRUNK, &[RELEASE]);
        let repositories = vec![
            api(),
            repository(LEGACY_ID, LEGACY_NAME, "./legacy"),
            repository(TOOLS_ID, TOOLS_NAME, "./tools"),
        ];

        let project = fixture
            .service
            .add_project(tag(), project_settings(repositories))
            .unwrap();

        assert_eq!(
            default_branches(&project),
            [
                (API_ID.to_string(), MAIN.to_string()),
                (LEGACY_ID.to_string(), MASTER.to_string()),
                (TOOLS_ID.to_string(), RELEASE.to_string()),
            ]
        );
    }

    #[test]
    fn skips_repositories_that_cant_be_opened() {
        let fixture = Fixture::without_project();
        let docs = repository(DOCS_ID, DOCS_NAME, "./docs");

        let project = fixture
            .service
            .add_project(tag(), project_settings(vec![api(), docs]))
            .unwrap();

        assert_eq!(
            default_branches(&project),
            [(API_ID.to_string(), MAIN.to_string())]
        );
    }

    #[test]
    fn refuses_a_tag_already_in_use_and_saves_nothing() {
        let fixture = Fixture::new();
        let stored_before = fixture.stored();

        let result = fixture
            .service
            .add_project(tag(), project_settings(vec![api()]));

        assert_eq!(
            result,
            Err(WorkspaceServiceError::ProjectAlreadyExists { tag: tag() })
        );
        assert_eq!(fixture.stored(), stored_before);
        assert_eq!(fixture.change_count(), 0);
    }

    #[test]
    fn refuses_settings_the_project_rejects() {
        let fixture = Fixture::without_project();
        let nameless = ProjectSettings {
            name: String::new(),
            ..project_settings(vec![api()])
        };

        let result = fixture.service.add_project(tag(), nameless);

        assert_eq!(
            result,
            Err(WorkspaceServiceError::Rejected(
                WorkspaceError::EmptyProjectName
            ))
        );
        assert_eq!(fixture.projects.find(&tag()).unwrap(), None);
    }
}

mod delete_project {
    use super::*;

    #[test]
    fn forgets_the_project_and_announces_it() {
        let fixture = Fixture::new();

        fixture.service.delete_project(&tag()).unwrap();

        assert_eq!(fixture.projects.find(&tag()).unwrap(), None);
        assert_eq!(fixture.events.changed_projects(), [tag()]);
    }

    #[test]
    fn leaves_the_branches_and_worktrees_on_disk() {
        let fixture = Fixture::new();
        fixture.create_login();

        fixture.service.delete_project(&tag()).unwrap();

        assert_eq!(
            fixture.checkout_of(API_NAME, LOGIN_BRANCH).as_deref(),
            Some(LOGIN_API_WORKTREE)
        );
    }

    #[test]
    fn succeeds_for_a_project_that_is_already_gone() {
        let fixture = Fixture::new();
        fixture.service.delete_project(&tag()).unwrap();

        let result = fixture.service.delete_project(&tag());

        assert_eq!(result, Ok(()));
    }
}

mod suggest_branch_name {
    use super::*;

    const TITLE: &str = "Login page";
    const SLUG: &str = "login-page";
    const FEATURE_PREFIX: &str = "feature";
    const REQUIRED_PREFIX: &str = "fix";
    const UNKNOWN_PREFIX: &str = "hotfix";
    const TITLE_WITHOUT_LETTERS: &str = "!!!";

    fn suggest(
        fixture: &Fixture,
        title: &str,
        prefix: Option<&str>,
    ) -> Result<String, WorkspaceServiceError> {
        fixture
            .service
            .suggest_branch_name(&tag(), title, prefix)
            .map(|branch| branch.to_string())
    }

    fn requiring_a_prefix() -> Fixture {
        let fixture = Fixture::new();
        let settings = ProjectSettings {
            branch_naming: BranchNamingPolicy::parse(&[REQUIRED_PREFIX], false).unwrap(),
            ..project_settings(vec![api(), web()])
        };
        fixture
            .projects
            .save(&fixture.stored().update_settings(settings).unwrap())
            .unwrap();
        fixture
    }

    fn rejected(error: WorkspaceError) -> Result<String, WorkspaceServiceError> {
        Err(WorkspaceServiceError::Rejected(error))
    }

    #[test]
    fn puts_the_chosen_prefix_before_the_slug() {
        let fixture = Fixture::new();

        let suggestion = suggest(&fixture, TITLE, Some(FEATURE_PREFIX));

        assert_eq!(suggestion, Ok(format!("{FEATURE_PREFIX}/{SLUG}")));
    }

    #[test]
    fn gives_the_slug_alone_when_no_prefix_is_allowed() {
        let fixture = Fixture::new();

        assert_eq!(suggest(&fixture, TITLE, None), Ok(SLUG.to_string()));
    }

    #[test]
    fn refuses_no_prefix_when_the_project_requires_one() {
        let fixture = requiring_a_prefix();

        assert_eq!(
            suggest(&fixture, TITLE, None),
            rejected(WorkspaceError::PrefixRequired)
        );
        assert_eq!(
            suggest(&fixture, TITLE, Some(REQUIRED_PREFIX)),
            Ok(format!("{REQUIRED_PREFIX}/{SLUG}"))
        );
    }

    #[test]
    fn refuses_a_prefix_the_project_doesnt_offer() {
        let fixture = Fixture::new();

        assert_eq!(
            suggest(&fixture, TITLE, Some(UNKNOWN_PREFIX)),
            rejected(WorkspaceError::UnknownPrefix {
                prefix: UNKNOWN_PREFIX.to_string()
            })
        );
    }

    #[test]
    fn refuses_a_title_that_gives_an_empty_slug() {
        let fixture = Fixture::new();

        assert_eq!(
            suggest(&fixture, TITLE_WITHOUT_LETTERS, Some(FEATURE_PREFIX)),
            rejected(WorkspaceError::EmptyBranchName {
                source: TITLE_WITHOUT_LETTERS.to_string()
            })
        );
    }
}
