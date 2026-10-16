use std::sync::{Arc, Mutex};

use super::registry::GitHostRegistry;
use super::service::GitHostingService;
use super::test_support::{
    account, open_pull_request, repository_remote, FakeHostTokens, FakeProjectRemotes, HostScript,
    RecordingEvents, BITBUCKET_LOGIN, GITHUB_LOGIN,
};
use crate::git_hosting::domain::capped::Capped;
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::hosted_pull_request::HostedPullRequest;
use crate::git_hosting::domain::project_pull_requests::{
    ProjectRepositoryRemote, PullRequestsStatus, RepositoryPullRequests,
};
use crate::git_hosting::infra::in_memory_hosted_accounts::InMemoryHostedAccounts;

const PROJECT_TAG: &str = "acme";
const UNKNOWN_PROJECT_TAG: &str = "unknown";
const API: &str = "api";
const WEB: &str = "web";
const API_GITHUB_REMOTE: &str = "git@github.com:acme/api.git";
const WEB_GITHUB_REMOTE: &str = "https://github.com/acme/web.git";
const API_BITBUCKET_REMOTE: &str = "https://bitbucket.org/acme/api.git";
const API_GITLAB_REMOTE: &str = "https://gitlab.com/acme/api.git";
const GITHUB_TOKEN: &str = "gho_token";
const BITBUCKET_SECRET: &str = "alice:api-token";
const MALFORMED_BITBUCKET_SECRET: &str = "no-separator";
const FIRST_NUMBER: u32 = 7;
const SECOND_NUMBER: u32 = 8;

struct Harness {
    host_tokens: FakeHostTokens,
    github: Arc<Mutex<HostScript>>,
    bitbucket: Arc<Mutex<HostScript>>,
}

impl Harness {
    fn signed_in_to(host: HostKind, secret: &str) -> Self {
        Self::with_tokens(FakeHostTokens::with_secret(host, secret))
    }

    fn with_tokens(host_tokens: FakeHostTokens) -> Self {
        Self {
            host_tokens,
            github: HostScript::shared(Ok(account(HostKind::Github, GITHUB_LOGIN))),
            bitbucket: HostScript::shared(Ok(account(HostKind::Bitbucket, BITBUCKET_LOGIN))),
        }
    }

    fn answer(
        &self,
        host: HostKind,
        repository_name: &str,
        result: Result<Capped<HostedPullRequest>, GitHostingError>,
    ) {
        let script = match host {
            HostKind::Github => &self.github,
            HostKind::Bitbucket => &self.bitbucket,
        };
        script
            .lock()
            .unwrap()
            .pull_requests_by_repository
            .insert(repository_name.to_string(), result);
    }

    async fn load(
        self,
        project_tag: &str,
        repositories: Vec<ProjectRepositoryRemote>,
    ) -> Result<Vec<RepositoryPullRequests>, GitHostingError> {
        let bitbucket_factory: super::registry::GitHostFactory = {
            let scripted = HostScript::factory(&self.bitbucket);
            Arc::new(move |secret| {
                // Like the real adapter: a malformed stored secret cannot sign in.
                if secret.bitbucket_credential().is_err() {
                    return Err(GitHostingError::NotConnected);
                }
                scripted(secret)
            })
        };
        let registry = GitHostRegistry::new()
            .register_host(HostKind::Github, HostScript::factory(&self.github))
            .register_host(HostKind::Bitbucket, bitbucket_factory);
        let service = GitHostingService::new(
            registry,
            Arc::new(self.host_tokens),
            Arc::new(InMemoryHostedAccounts::default()),
            Arc::new(RecordingEvents::default()),
            Arc::new(FakeProjectRemotes::project(PROJECT_TAG, repositories)),
        );
        service.project_open_pull_requests(project_tag).await
    }

    async fn statuses(self, repositories: Vec<ProjectRepositoryRemote>) -> Vec<PullRequestsStatus> {
        self.load(PROJECT_TAG, repositories)
            .await
            .unwrap()
            .into_iter()
            .map(|repository| repository.status)
            .collect()
    }
}

fn listed(numbers: &[u32], truncated: bool) -> Capped<HostedPullRequest> {
    Capped {
        items: numbers.iter().copied().map(open_pull_request).collect(),
        truncated,
    }
}

fn github_harness() -> Harness {
    Harness::signed_in_to(HostKind::Github, GITHUB_TOKEN)
}

#[tokio::test]
async fn an_unknown_project_is_not_found() {
    let result = github_harness().load(UNKNOWN_PROJECT_TAG, vec![]).await;

    assert_eq!(result, Err(GitHostingError::NotFound));
}

#[tokio::test]
async fn lists_the_open_pull_requests_of_a_github_repository() {
    let harness = github_harness();
    harness.answer(
        HostKind::Github,
        API,
        Ok(listed(&[FIRST_NUMBER, SECOND_NUMBER], false)),
    );

    let repositories = harness
        .load(
            PROJECT_TAG,
            vec![repository_remote(API, Some(API_GITHUB_REMOTE))],
        )
        .await
        .unwrap();

    assert_eq!(
        repositories,
        vec![RepositoryPullRequests {
            repository_id: repository_remote(API, None).repository_id,
            repository_name: API.to_string(),
            host: Some(HostKind::Github),
            status: PullRequestsStatus::Ok(listed(&[FIRST_NUMBER, SECOND_NUMBER], false)),
        }]
    );
}

#[tokio::test]
async fn lists_the_open_pull_requests_of_a_bitbucket_repository() {
    let harness = Harness::signed_in_to(HostKind::Bitbucket, BITBUCKET_SECRET);
    harness.answer(HostKind::Bitbucket, API, Ok(listed(&[FIRST_NUMBER], false)));

    let statuses = harness
        .statuses(vec![repository_remote(API, Some(API_BITBUCKET_REMOTE))])
        .await;

    assert_eq!(
        statuses,
        vec![PullRequestsStatus::Ok(listed(&[FIRST_NUMBER], false))]
    );
}

#[tokio::test]
async fn passes_a_list_cut_short_through() {
    let harness = github_harness();
    harness.answer(HostKind::Github, API, Ok(listed(&[FIRST_NUMBER], true)));

    let statuses = harness
        .statuses(vec![repository_remote(API, Some(API_GITHUB_REMOTE))])
        .await;

    assert_eq!(
        statuses,
        vec![PullRequestsStatus::Ok(listed(&[FIRST_NUMBER], true))]
    );
}

#[tokio::test]
async fn a_repository_without_remote_has_no_remote() {
    let repositories = github_harness()
        .load(PROJECT_TAG, vec![repository_remote(API, None)])
        .await
        .unwrap();

    assert_eq!(repositories[0].host, None);
    assert_eq!(repositories[0].status, PullRequestsStatus::NoRemote);
}

#[tokio::test]
async fn a_remote_on_another_host_is_unsupported() {
    let repositories = github_harness()
        .load(
            PROJECT_TAG,
            vec![repository_remote(API, Some(API_GITLAB_REMOTE))],
        )
        .await
        .unwrap();

    assert_eq!(repositories[0].host, None);
    assert_eq!(repositories[0].status, PullRequestsStatus::UnsupportedHost);
}

#[tokio::test]
async fn a_host_without_a_secret_is_not_connected() {
    let repositories = Harness::with_tokens(FakeHostTokens::default())
        .load(
            PROJECT_TAG,
            vec![repository_remote(API, Some(API_GITHUB_REMOTE))],
        )
        .await
        .unwrap();

    assert_eq!(repositories[0].host, Some(HostKind::Github));
    assert_eq!(repositories[0].status, PullRequestsStatus::NotConnected);
}

#[tokio::test]
async fn a_secret_that_cannot_sign_in_is_not_connected() {
    let statuses = Harness::signed_in_to(HostKind::Bitbucket, MALFORMED_BITBUCKET_SECRET)
        .statuses(vec![repository_remote(API, Some(API_BITBUCKET_REMOTE))])
        .await;

    assert_eq!(statuses, vec![PullRequestsStatus::NotConnected]);
}

#[tokio::test]
async fn a_failing_host_fails_the_repository_with_its_error() {
    let harness = github_harness();
    harness.answer(HostKind::Github, API, Err(GitHostingError::Unauthorized));

    let statuses = harness
        .statuses(vec![repository_remote(API, Some(API_GITHUB_REMOTE))])
        .await;

    assert_eq!(
        statuses,
        vec![PullRequestsStatus::Failed(GitHostingError::Unauthorized)]
    );
}

#[tokio::test]
async fn an_unreadable_keychain_fails_the_repository() {
    let statuses = Harness::with_tokens(FakeHostTokens::unavailable())
        .statuses(vec![repository_remote(API, Some(API_GITHUB_REMOTE))])
        .await;

    assert_eq!(
        statuses,
        vec![PullRequestsStatus::Failed(GitHostingError::Storage)]
    );
}

#[tokio::test]
async fn one_failing_repository_leaves_the_others_in_project_order() {
    let harness = github_harness();
    harness.answer(HostKind::Github, API, Err(GitHostingError::Network));
    harness.answer(HostKind::Github, WEB, Ok(listed(&[FIRST_NUMBER], false)));

    let repositories = harness
        .load(
            PROJECT_TAG,
            vec![
                repository_remote(API, Some(API_GITHUB_REMOTE)),
                repository_remote(WEB, Some(WEB_GITHUB_REMOTE)),
            ],
        )
        .await
        .unwrap();

    let names_and_statuses: Vec<(&str, &PullRequestsStatus)> = repositories
        .iter()
        .map(|repository| (repository.repository_name.as_str(), &repository.status))
        .collect();
    assert_eq!(
        names_and_statuses,
        vec![
            (API, &PullRequestsStatus::Failed(GitHostingError::Network)),
            (WEB, &PullRequestsStatus::Ok(listed(&[FIRST_NUMBER], false))),
        ]
    );
}
