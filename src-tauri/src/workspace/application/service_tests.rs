//! Service tests on the in-memory project repository, the in-memory git fake
//! and the recording files and events fakes.
use std::path::PathBuf;

use super::*;
use crate::git::infra::in_memory::{InMemoryGitRepositories, Operation};
use crate::workspace::application::error::CheckoutHolder;
use crate::workspace::domain::branch_naming::BranchNamingPolicy;
use crate::workspace::domain::context_branch::ContextCheckout;
use crate::workspace::domain::error::WorkspaceError;
use crate::workspace::domain::project::ProjectSettings;
use crate::workspace::domain::test_support::{branch_name, repository, repository_id};
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
        git.add_repository(&path_of(API_NAME), &branch_name(MAIN));
        git.add_repository(&path_of(WEB_NAME), &branch_name(MAIN));
        let service =
            WorkspaceService::new(projects.clone(), git.clone(), files.clone(), events.clone());
        Self {
            projects,
            git,
            files,
            events,
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
