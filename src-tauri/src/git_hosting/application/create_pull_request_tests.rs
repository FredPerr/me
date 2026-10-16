use std::sync::{Arc, Mutex};

use super::registry::GitHostRegistry;
use super::service::GitHostingService;
use super::test_support::{
    account, created_pull_request, repository_remote, worktree_path_of, FakeClock, FakeHostTokens,
    FakeProjectRemotes, FakeWorktreePushes, HostScript, RecordingEvents, RepositoryBranch, Step,
    Steps, BITBUCKET_LOGIN, CREATED_NUMBER, GITHUB_LOGIN,
};
use crate::git::domain::remote_url::RemoteUrl;
use crate::git_hosting::domain::error::{GitHostingError, InputField, PushFailureReason};
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::new_pull_request::{CreatedPullRequest, NewPullRequest};
use crate::git_hosting::domain::ports::PushFailure;
use crate::git_hosting::domain::project_pull_requests::ProjectRepositoryRemote;
use crate::git_hosting::infra::in_memory_hosted_accounts::InMemoryHostedAccounts;

const PROJECT_TAG: &str = "acme";
const UNKNOWN_PROJECT_TAG: &str = "unknown";
const UNKNOWN_REPOSITORY_ID: &str = "unknown-id";
const API: &str = "api";
const API_GITHUB_REMOTE: &str = "git@github.com:acme/api.git";
const API_GITLAB_REMOTE: &str = "https://gitlab.com/acme/api.git";
const GITHUB_TOKEN: &str = "gho_token";
const LOGIN_BRANCH: &str = "feature/login";
const UNUSED_BRANCH: &str = "feature/unused";
const BASE_BRANCH: &str = "main";
const TITLE: &str = "Add the login page";
const PADDED_TITLE: &str = "  Add the login page ";
const BLANK_TITLE: &str = " ";
const BODY: &str = "Adds a login page.";

fn new_pull_request(title: &str, head: &str, base: &str) -> NewPullRequest {
    NewPullRequest {
        title: title.to_string(),
        body: BODY.to_string(),
        head: head.to_string(),
        base: base.to_string(),
        draft: true,
    }
}

fn login_pull_request() -> NewPullRequest {
    new_pull_request(TITLE, LOGIN_BRANCH, BASE_BRANCH)
}

fn api_on_login(remote: Option<&str>) -> ProjectRepositoryRemote {
    ProjectRepositoryRemote {
        context_branches: vec![LOGIN_BRANCH.to_string()],
        ..repository_remote(API, remote)
    }
}

fn api_id() -> String {
    repository_remote(API, None).repository_id
}

struct Harness {
    service: GitHostingService,
    github: Arc<Mutex<HostScript>>,
    steps: Steps,
}

impl Harness {
    fn new(
        host_tokens: FakeHostTokens,
        repository: ProjectRepositoryRemote,
        push_failure: Option<PushFailure>,
    ) -> Self {
        let github = HostScript::shared(Ok(account(HostKind::Github, GITHUB_LOGIN)));
        let bitbucket = HostScript::shared(Ok(account(HostKind::Bitbucket, BITBUCKET_LOGIN)));
        let steps = github.lock().unwrap().steps.clone();
        let registry = GitHostRegistry::new()
            .register_host(HostKind::Github, HostScript::factory(&github))
            .register_host(HostKind::Bitbucket, HostScript::factory(&bitbucket));
        let service = GitHostingService::new(
            registry,
            Arc::new(host_tokens),
            Arc::new(InMemoryHostedAccounts::default()),
            Arc::new(RecordingEvents::default()),
            Arc::new(FakeProjectRemotes::project(PROJECT_TAG, vec![repository])),
            Arc::new(FakeWorktreePushes::recording(&steps, push_failure)),
            Arc::new(FakeClock::default()),
        );
        Self {
            service,
            github,
            steps,
        }
    }

    fn signed_in_to_github(repository: ProjectRepositoryRemote) -> Self {
        Self::new(
            FakeHostTokens::with_secret(HostKind::Github, GITHUB_TOKEN),
            repository,
            None,
        )
    }

    /// api on GitHub, signed in, with a context on the login branch.
    fn github_api_on_login() -> Self {
        Self::signed_in_to_github(api_on_login(Some(API_GITHUB_REMOTE)))
    }

    fn pushing_fails_with(failure: PushFailure) -> Self {
        Self::new(
            FakeHostTokens::with_secret(HostKind::Github, GITHUB_TOKEN),
            api_on_login(Some(API_GITHUB_REMOTE)),
            Some(failure),
        )
    }

    async fn create(
        &self,
        pull_request: NewPullRequest,
    ) -> Result<CreatedPullRequest, GitHostingError> {
        self.service
            .create_pull_request(PROJECT_TAG, &api_id(), pull_request)
            .await
    }

    fn steps(&self) -> Vec<Step> {
        self.steps.lock().unwrap().clone()
    }

    async fn look_up_context_branches(&self) {
        self.service
            .context_branch_pull_requests(PROJECT_TAG)
            .await
            .unwrap();
    }

    fn branch_lookups(&self) -> Vec<RepositoryBranch> {
        self.github.lock().unwrap().branch_lookups.clone()
    }
}

fn login_lookup() -> RepositoryBranch {
    (API.to_string(), LOGIN_BRANCH.to_string())
}

#[tokio::test]
async fn pushes_the_branch_then_creates_the_pull_request() {
    let harness = Harness::github_api_on_login();

    let created = harness
        .create(new_pull_request(PADDED_TITLE, LOGIN_BRANCH, BASE_BRANCH))
        .await;

    assert_eq!(created, Ok(created_pull_request(CREATED_NUMBER)));
    assert_eq!(
        harness.steps(),
        vec![
            Step::Pushed {
                worktree_path: worktree_path_of(API, LOGIN_BRANCH),
            },
            Step::Created {
                repository_name: API.to_string(),
                pull_request: login_pull_request(),
            },
        ]
    );
}

#[tokio::test]
async fn a_skipped_push_stops_before_creating() {
    let harness = Harness::pushing_fails_with(PushFailure::Skipped);

    let created = harness.create(login_pull_request()).await;

    assert_eq!(created, Err(GitHostingError::PushSkipped));
    assert_eq!(harness.steps(), vec![]);
}

#[tokio::test]
async fn a_failed_push_stops_before_creating() {
    let harness = Harness::pushing_fails_with(PushFailure::Failed(PushFailureReason::Unauthorized));

    let created = harness.create(login_pull_request()).await;

    assert_eq!(
        created,
        Err(GitHostingError::PushFailed {
            reason: PushFailureReason::Unauthorized
        })
    );
    assert_eq!(harness.steps(), vec![]);
}

#[tokio::test]
async fn a_failed_creation_is_passed_through() {
    let harness = Harness::github_api_on_login();
    harness.github.lock().unwrap().create_result = Err(GitHostingError::Forbidden);

    let created = harness.create(login_pull_request()).await;

    assert_eq!(created, Err(GitHostingError::Forbidden));
}

#[tokio::test]
async fn a_repository_on_an_unsupported_host_pushes_nothing() {
    let harness = Harness::signed_in_to_github(api_on_login(Some(API_GITLAB_REMOTE)));

    let created = harness.create(login_pull_request()).await;

    assert_eq!(created, Err(GitHostingError::UnsupportedHost));
    assert_eq!(harness.steps(), vec![]);
}

#[tokio::test]
async fn a_repository_without_remote_has_no_remote() {
    let harness = Harness::signed_in_to_github(api_on_login(None));

    let created = harness.create(login_pull_request()).await;

    assert_eq!(created, Err(GitHostingError::NoRemote));
}

#[tokio::test]
async fn a_git_host_without_a_secret_is_not_connected() {
    let harness = Harness::new(
        FakeHostTokens::default(),
        api_on_login(Some(API_GITHUB_REMOTE)),
        None,
    );

    let created = harness.create(login_pull_request()).await;

    assert_eq!(created, Err(GitHostingError::NotConnected));
    assert_eq!(harness.steps(), vec![]);
}

#[tokio::test]
async fn an_unknown_project_or_repository_is_not_found() {
    let harness = Harness::github_api_on_login();

    for (project_tag, repository_id) in [
        (UNKNOWN_PROJECT_TAG, api_id()),
        (PROJECT_TAG, UNKNOWN_REPOSITORY_ID.to_string()),
    ] {
        let created = harness
            .service
            .create_pull_request(project_tag, &repository_id, login_pull_request())
            .await;

        assert_eq!(created, Err(GitHostingError::NotFound), "{project_tag}");
    }
}

#[tokio::test]
async fn a_branch_no_context_uses_is_not_found() {
    let harness = Harness::github_api_on_login();

    let created = harness
        .create(new_pull_request(TITLE, UNUSED_BRANCH, BASE_BRANCH))
        .await;

    assert_eq!(created, Err(GitHostingError::NotFound));
}

#[tokio::test]
async fn a_blank_title_pushes_nothing() {
    let harness = Harness::github_api_on_login();

    let created = harness
        .create(new_pull_request(BLANK_TITLE, LOGIN_BRANCH, BASE_BRANCH))
        .await;

    assert_eq!(
        created,
        Err(GitHostingError::invalid_input(InputField::Title))
    );
    assert_eq!(harness.steps(), vec![]);
}

#[tokio::test]
async fn merging_a_branch_into_itself_is_refused() {
    let harness = Harness::github_api_on_login();

    let created = harness
        .create(new_pull_request(TITLE, LOGIN_BRANCH, LOGIN_BRANCH))
        .await;

    assert_eq!(
        created,
        Err(GitHostingError::invalid_input(InputField::BaseBranch))
    );
}

#[tokio::test]
async fn creating_a_pull_request_asks_the_git_host_again_for_its_branch() {
    let harness = Harness::github_api_on_login();
    harness.look_up_context_branches().await;

    harness.create(login_pull_request()).await.unwrap();
    harness.look_up_context_branches().await;

    assert_eq!(
        harness.branch_lookups(),
        vec![login_lookup(), login_lookup()]
    );
}

#[tokio::test]
async fn pushing_a_branch_asks_the_git_host_again_for_it() {
    let harness = Harness::github_api_on_login();
    harness.look_up_context_branches().await;

    harness
        .service
        .branch_pushed(&RemoteUrl::new(API_GITHUB_REMOTE), LOGIN_BRANCH);
    harness.look_up_context_branches().await;

    assert_eq!(
        harness.branch_lookups(),
        vec![login_lookup(), login_lookup()]
    );
}
