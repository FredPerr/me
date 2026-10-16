use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::branch_pull_request_cache::BRANCH_LOOKUP_LIFETIME;
use super::registry::GitHostRegistry;
use super::service::GitHostingService;
use super::test_support::{
    account, open_pull_request, repository_remote, FakeClock, FakeHostTokens, FakeProjectRemotes,
    FakeWorktreePushes, HostScript, RecordingEvents, RepositoryBranch, BITBUCKET_LOGIN,
    GITHUB_LOGIN,
};
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::hosted_pull_request::{HostedPullRequest, PullRequestState};
use crate::git_hosting::domain::hosted_repository::HostedRepository;
use crate::git_hosting::domain::project_pull_requests::{
    BranchPullRequest, BranchPullRequestStatus, ProjectRepositoryRemote, PullRequestsStatus,
};
use crate::git_hosting::infra::in_memory_hosted_accounts::InMemoryHostedAccounts;

const PROJECT_TAG: &str = "acme";
const UNKNOWN_PROJECT_TAG: &str = "unknown";
const API: &str = "api";
const WEB: &str = "web";
const OWNER: &str = "acme";
const API_GITHUB_REMOTE: &str = "git@github.com:acme/api.git";
const WEB_GITHUB_REMOTE: &str = "https://github.com/acme/web.git";
const API_BITBUCKET_REMOTE: &str = "https://bitbucket.org/acme/api.git";
const API_GITLAB_REMOTE: &str = "https://gitlab.com/acme/api.git";
const GITHUB_TOKEN: &str = "gho_token";
const BITBUCKET_SECRET: &str = "alice:api-token";
const LOGIN_BRANCH: &str = "feature/login";
const SEARCH_BRANCH: &str = "feature/search";
const LOGIN_NUMBER: u32 = 7;
const REOPENED_LOGIN_NUMBER: u32 = 9;
const SEARCH_NUMBER: u32 = 8;
const ONE_SECOND: Duration = Duration::from_secs(1);

fn pull_request_from(branch: &str, number: u32) -> HostedPullRequest {
    HostedPullRequest {
        head_branch: branch.to_string(),
        state: PullRequestState::Merged,
        ..open_pull_request(number)
    }
}

fn on_branches(
    name: &str,
    remote: Option<&str>,
    context_branches: &[&str],
) -> ProjectRepositoryRemote {
    ProjectRepositoryRemote {
        context_branches: context_branches
            .iter()
            .map(|branch| branch.to_string())
            .collect(),
        ..repository_remote(name, remote)
    }
}

fn github_api(owner: &str) -> HostedRepository {
    HostedRepository {
        host: HostKind::Github,
        owner: owner.to_string(),
        name: API.to_string(),
    }
}

fn repository_branch(repository_name: &str, branch: &str) -> RepositoryBranch {
    (repository_name.to_string(), branch.to_string())
}

struct Harness {
    service: GitHostingService,
    clock: Arc<FakeClock>,
    github: Arc<Mutex<HostScript>>,
    bitbucket: Arc<Mutex<HostScript>>,
}

impl Harness {
    fn new(host_tokens: FakeHostTokens, repositories: Vec<ProjectRepositoryRemote>) -> Self {
        let clock = Arc::new(FakeClock::default());
        let github = HostScript::shared(Ok(account(HostKind::Github, GITHUB_LOGIN)));
        let bitbucket = HostScript::shared(Ok(account(HostKind::Bitbucket, BITBUCKET_LOGIN)));
        let registry = GitHostRegistry::new()
            .register_host(HostKind::Github, HostScript::factory(&github))
            .register_host(HostKind::Bitbucket, HostScript::factory(&bitbucket));
        let service = GitHostingService::new(
            registry,
            Arc::new(host_tokens),
            Arc::new(InMemoryHostedAccounts::default()),
            Arc::new(RecordingEvents::default()),
            Arc::new(FakeProjectRemotes::project(PROJECT_TAG, repositories)),
            Arc::new(FakeWorktreePushes::default()),
            clock.clone(),
        );
        Self {
            service,
            clock,
            github,
            bitbucket,
        }
    }

    fn signed_in_to_github(repositories: Vec<ProjectRepositoryRemote>) -> Self {
        Self::new(
            FakeHostTokens::with_secret(HostKind::Github, GITHUB_TOKEN),
            repositories,
        )
    }

    /// api on GitHub, with contexts on the login and search branches.
    fn github_api_on_login_and_search() -> Self {
        Self::signed_in_to_github(vec![on_branches(
            API,
            Some(API_GITHUB_REMOTE),
            &[LOGIN_BRANCH, SEARCH_BRANCH],
        )])
    }

    fn script(&self, host: HostKind) -> &Arc<Mutex<HostScript>> {
        match host {
            HostKind::Github => &self.github,
            HostKind::Bitbucket => &self.bitbucket,
        }
    }

    fn answer(
        &self,
        host: HostKind,
        repository_name: &str,
        branch: &str,
        result: Result<Option<HostedPullRequest>, GitHostingError>,
    ) {
        self.script(host)
            .lock()
            .unwrap()
            .pull_request_by_branch
            .insert(repository_branch(repository_name, branch), result);
    }

    fn lookups(&self, host: HostKind) -> Vec<RepositoryBranch> {
        self.script(host).lock().unwrap().branch_lookups.clone()
    }

    async fn load(&self) -> Vec<BranchPullRequest> {
        self.service
            .context_branch_pull_requests(PROJECT_TAG)
            .await
            .unwrap()
    }

    async fn statuses(&self) -> Vec<(String, BranchPullRequestStatus)> {
        self.load()
            .await
            .into_iter()
            .map(|found| (found.branch, found.status))
            .collect()
    }

    async fn status_of_only_branch(&self) -> BranchPullRequestStatus {
        let mut found = self.load().await;
        assert_eq!(found.len(), 1);
        found.remove(0).status
    }
}

fn found(
    branch: &str,
    pull_request: Option<HostedPullRequest>,
) -> (String, BranchPullRequestStatus) {
    (branch.to_string(), PullRequestsStatus::Ok(pull_request))
}

#[tokio::test]
async fn an_unknown_project_is_not_found() {
    let harness = Harness::github_api_on_login_and_search();

    let result = harness
        .service
        .context_branch_pull_requests(UNKNOWN_PROJECT_TAG)
        .await;

    assert_eq!(result, Err(GitHostingError::NotFound));
}

#[tokio::test]
async fn finds_the_pull_request_of_each_context_branch() {
    let harness = Harness::github_api_on_login_and_search();
    let login = pull_request_from(LOGIN_BRANCH, LOGIN_NUMBER);
    harness.answer(HostKind::Github, API, LOGIN_BRANCH, Ok(Some(login.clone())));

    let found = harness.load().await;

    assert_eq!(
        found,
        vec![
            BranchPullRequest {
                repository_id: repository_remote(API, None).repository_id,
                branch: LOGIN_BRANCH.to_string(),
                host: Some(HostKind::Github),
                status: PullRequestsStatus::Ok(Some(login)),
            },
            BranchPullRequest {
                repository_id: repository_remote(API, None).repository_id,
                branch: SEARCH_BRANCH.to_string(),
                host: Some(HostKind::Github),
                status: PullRequestsStatus::Ok(None),
            },
        ]
    );
}

#[tokio::test]
async fn finds_the_pull_request_of_a_bitbucket_branch() {
    let harness = Harness::new(
        FakeHostTokens::with_secret(HostKind::Bitbucket, BITBUCKET_SECRET),
        vec![on_branches(
            API,
            Some(API_BITBUCKET_REMOTE),
            &[LOGIN_BRANCH],
        )],
    );
    let login = pull_request_from(LOGIN_BRANCH, LOGIN_NUMBER);
    harness.answer(
        HostKind::Bitbucket,
        API,
        LOGIN_BRANCH,
        Ok(Some(login.clone())),
    );

    assert_eq!(
        harness.status_of_only_branch().await,
        PullRequestsStatus::Ok(Some(login))
    );
}

#[tokio::test]
async fn keeps_the_project_order_of_repositories() {
    let harness = Harness::signed_in_to_github(vec![
        on_branches(WEB, Some(WEB_GITHUB_REMOTE), &[SEARCH_BRANCH]),
        on_branches(API, Some(API_GITHUB_REMOTE), &[LOGIN_BRANCH]),
    ]);

    let repository_branches: Vec<(String, String)> = harness
        .load()
        .await
        .into_iter()
        .map(|found| (found.repository_id, found.branch))
        .collect();

    assert_eq!(
        repository_branches,
        vec![
            (
                repository_remote(WEB, None).repository_id,
                SEARCH_BRANCH.to_string()
            ),
            (
                repository_remote(API, None).repository_id,
                LOGIN_BRANCH.to_string()
            ),
        ]
    );
}

#[tokio::test]
async fn looks_up_a_branch_used_by_several_contexts_once() {
    let harness = Harness::signed_in_to_github(vec![on_branches(
        API,
        Some(API_GITHUB_REMOTE),
        &[LOGIN_BRANCH, SEARCH_BRANCH, LOGIN_BRANCH],
    )]);

    let statuses = harness.statuses().await;

    assert_eq!(
        statuses,
        vec![found(LOGIN_BRANCH, None), found(SEARCH_BRANCH, None)]
    );
    assert_eq!(
        harness.lookups(HostKind::Github),
        vec![
            repository_branch(API, LOGIN_BRANCH),
            repository_branch(API, SEARCH_BRANCH),
        ]
    );
}

#[tokio::test]
async fn a_repository_no_context_uses_has_no_branches() {
    let harness =
        Harness::signed_in_to_github(vec![on_branches(API, Some(API_GITHUB_REMOTE), &[])]);

    assert_eq!(harness.load().await, vec![]);
}

#[tokio::test]
async fn each_branch_of_a_repository_without_remote_has_no_remote() {
    let harness =
        Harness::signed_in_to_github(vec![on_branches(API, None, &[LOGIN_BRANCH, SEARCH_BRANCH])]);

    let found = harness.load().await;

    assert_eq!(found[0].host, None);
    assert_eq!(
        found
            .into_iter()
            .map(|found| found.status)
            .collect::<Vec<_>>(),
        vec![PullRequestsStatus::NoRemote, PullRequestsStatus::NoRemote]
    );
}

#[tokio::test]
async fn a_branch_on_another_host_is_unsupported() {
    let harness = Harness::signed_in_to_github(vec![on_branches(
        API,
        Some(API_GITLAB_REMOTE),
        &[LOGIN_BRANCH],
    )]);

    assert_eq!(
        harness.status_of_only_branch().await,
        PullRequestsStatus::UnsupportedHost
    );
}

#[tokio::test]
async fn a_branch_on_a_host_without_a_secret_is_not_connected() {
    let harness = Harness::new(
        FakeHostTokens::default(),
        vec![on_branches(API, Some(API_GITHUB_REMOTE), &[LOGIN_BRANCH])],
    );

    let found = harness.load().await;

    assert_eq!(found[0].host, Some(HostKind::Github));
    assert_eq!(found[0].status, PullRequestsStatus::NotConnected);
}

#[tokio::test]
async fn an_unreadable_keychain_fails_each_branch() {
    let harness = Harness::new(
        FakeHostTokens::unavailable(),
        vec![on_branches(API, Some(API_GITHUB_REMOTE), &[LOGIN_BRANCH])],
    );

    assert_eq!(
        harness.status_of_only_branch().await,
        PullRequestsStatus::Failed(GitHostingError::Storage)
    );
}

#[tokio::test]
async fn one_failing_branch_leaves_the_others() {
    let harness = Harness::github_api_on_login_and_search();
    harness.answer(
        HostKind::Github,
        API,
        LOGIN_BRANCH,
        Err(GitHostingError::Forbidden),
    );
    let search = pull_request_from(SEARCH_BRANCH, SEARCH_NUMBER);
    harness.answer(
        HostKind::Github,
        API,
        SEARCH_BRANCH,
        Ok(Some(search.clone())),
    );

    assert_eq!(
        harness.statuses().await,
        vec![
            (
                LOGIN_BRANCH.to_string(),
                PullRequestsStatus::Failed(GitHostingError::Forbidden)
            ),
            found(SEARCH_BRANCH, Some(search)),
        ]
    );
}

mod cache {
    use super::*;

    fn github_api_on_login() -> Harness {
        Harness::signed_in_to_github(vec![on_branches(
            API,
            Some(API_GITHUB_REMOTE),
            &[LOGIN_BRANCH],
        )])
    }

    fn login_lookup_count(harness: &Harness) -> usize {
        harness.lookups(HostKind::Github).len()
    }

    #[tokio::test]
    async fn reuses_an_answer_within_the_lookup_lifetime() {
        let harness = github_api_on_login();
        let login = pull_request_from(LOGIN_BRANCH, LOGIN_NUMBER);
        harness.answer(HostKind::Github, API, LOGIN_BRANCH, Ok(Some(login.clone())));
        harness.load().await;
        harness.answer(
            HostKind::Github,
            API,
            LOGIN_BRANCH,
            Ok(Some(pull_request_from(LOGIN_BRANCH, REOPENED_LOGIN_NUMBER))),
        );

        harness.clock.advance(BRANCH_LOOKUP_LIFETIME - ONE_SECOND);

        assert_eq!(
            harness.status_of_only_branch().await,
            PullRequestsStatus::Ok(Some(login))
        );
        assert_eq!(login_lookup_count(&harness), 1);
    }

    #[tokio::test]
    async fn reuses_a_branch_without_pull_request() {
        let harness = github_api_on_login();
        harness.load().await;

        harness.load().await;

        assert_eq!(login_lookup_count(&harness), 1);
    }

    #[tokio::test]
    async fn asks_again_once_the_lookup_lifetime_is_over() {
        let harness = github_api_on_login();
        harness.load().await;
        let reopened = pull_request_from(LOGIN_BRANCH, REOPENED_LOGIN_NUMBER);
        harness.answer(
            HostKind::Github,
            API,
            LOGIN_BRANCH,
            Ok(Some(reopened.clone())),
        );

        harness.clock.advance(BRANCH_LOOKUP_LIFETIME);

        assert_eq!(
            harness.status_of_only_branch().await,
            PullRequestsStatus::Ok(Some(reopened))
        );
        assert_eq!(login_lookup_count(&harness), 2);
    }

    #[tokio::test]
    async fn does_not_keep_failures() {
        let harness = github_api_on_login();
        harness.answer(
            HostKind::Github,
            API,
            LOGIN_BRANCH,
            Err(GitHostingError::Network),
        );
        harness.load().await;
        harness.answer(HostKind::Github, API, LOGIN_BRANCH, Ok(None));

        assert_eq!(
            harness.status_of_only_branch().await,
            PullRequestsStatus::Ok(None)
        );
        assert_eq!(login_lookup_count(&harness), 2);
    }

    #[tokio::test]
    async fn asks_again_for_a_forgotten_branch() {
        let harness = github_api_on_login();
        harness.load().await;
        let created = pull_request_from(LOGIN_BRANCH, LOGIN_NUMBER);
        harness.answer(
            HostKind::Github,
            API,
            LOGIN_BRANCH,
            Ok(Some(created.clone())),
        );

        harness
            .service
            .forget_branch(&github_api(OWNER), LOGIN_BRANCH);

        assert_eq!(
            harness.status_of_only_branch().await,
            PullRequestsStatus::Ok(Some(created))
        );
        assert_eq!(login_lookup_count(&harness), 2);
    }

    #[tokio::test]
    async fn forgetting_a_branch_keeps_the_others() {
        let harness = Harness::github_api_on_login_and_search();
        harness.load().await;

        harness
            .service
            .forget_branch(&github_api(OWNER), LOGIN_BRANCH);
        harness.load().await;

        assert_eq!(
            harness.lookups(HostKind::Github),
            vec![
                repository_branch(API, LOGIN_BRANCH),
                repository_branch(API, SEARCH_BRANCH),
                repository_branch(API, LOGIN_BRANCH),
            ]
        );
    }

    #[tokio::test]
    async fn a_branch_of_another_repository_is_looked_up_on_its_own() {
        let harness = Harness::signed_in_to_github(vec![
            on_branches(API, Some(API_GITHUB_REMOTE), &[LOGIN_BRANCH]),
            on_branches(WEB, Some(WEB_GITHUB_REMOTE), &[LOGIN_BRANCH]),
        ]);

        harness.load().await;

        assert_eq!(
            harness.lookups(HostKind::Github),
            vec![
                repository_branch(API, LOGIN_BRANCH),
                repository_branch(WEB, LOGIN_BRANCH),
            ]
        );
    }
}
