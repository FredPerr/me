use async_trait::async_trait;

use super::client::{GithubClient, ACCESS_TOKEN_URL, API_BASE, DEVICE_CODE_URL, USER_URL};
use super::wire::{AccessTokenResponse, DeviceCodeResponse, PullRequestResponse, UserResponse};
use super::{mapping, pull_request_mapping};
use crate::git_hosting::domain::capped::Capped;
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::hosted_account::HostedAccount;
use crate::git_hosting::domain::hosted_pull_request::HostedPullRequest;
use crate::git_hosting::domain::hosted_repository::HostedRepository;
use crate::git_hosting::domain::ports::{BrowserSignIn, GitHost};
use crate::git_hosting::domain::sign_in::{BrowserSignInStep, SignInPoll};
use crate::git_hosting::infra::paging::{api_url, follow_pages, Page};

/// Client id of the "me" GitHub App. It is public (it ships in every OAuth
/// redirect) and the device flow needs no client secret.
const CLIENT_ID: &str = "Iv23lilfZx8UAX1Afhti";
const DEVICE_CODE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const OPEN_PULL_REQUESTS_QUERY: [(&str, &str); 4] = [
    ("state", "open"),
    ("sort", "updated"),
    ("direction", "desc"),
    ("per_page", "100"),
];

pub struct GithubHost {
    client: GithubClient,
    token: HostSecret,
}

impl GithubHost {
    pub fn new(client: GithubClient, token: HostSecret) -> Self {
        Self { client, token }
    }
}

#[async_trait]
impl GitHost for GithubHost {
    async fn current_account(&self) -> Result<HostedAccount, GitHostingError> {
        let user: UserResponse = self
            .client
            .get_json(USER_URL, &self.token, "current_account")
            .await?;
        Ok(mapping::account(user))
    }

    async fn list_open_pull_requests(
        &self,
        repository: &HostedRepository,
    ) -> Result<Capped<HostedPullRequest>, GitHostingError> {
        let first_url = api_url(
            API_BASE,
            &["repos", &repository.owner, &repository.name, "pulls"],
            &OPEN_PULL_REQUESTS_QUERY,
        )?;
        follow_pages(first_url, API_BASE, |url| async move {
            let (pull_requests, link_header): (Vec<PullRequestResponse>, _) = self
                .client
                .get_page(&url, &self.token, "list_open_pull_requests")
                .await?;
            Ok(Page {
                items: pull_requests
                    .into_iter()
                    .map(pull_request_mapping::pull_request)
                    .collect(),
                next_url: pull_request_mapping::next_page_url(link_header.as_deref()),
            })
        })
        .await
    }
}

/// GitHub's OAuth device flow: the user enters a code at github.com/login/device.
pub struct GithubBrowserSignIn {
    client: GithubClient,
}

impl GithubBrowserSignIn {
    pub fn new(client: GithubClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl BrowserSignIn for GithubBrowserSignIn {
    async fn start(&self) -> Result<BrowserSignInStep, GitHostingError> {
        let response: DeviceCodeResponse = self
            .client
            .post_form(
                DEVICE_CODE_URL,
                &[("client_id", CLIENT_ID)],
                "start_sign_in",
            )
            .await?;
        Ok(mapping::browser_sign_in_step(response))
    }

    async fn poll(&self, device_code: &str) -> Result<SignInPoll, GitHostingError> {
        let response: AccessTokenResponse = self
            .client
            .post_form(
                ACCESS_TOKEN_URL,
                &[
                    ("client_id", CLIENT_ID),
                    ("device_code", device_code),
                    ("grant_type", DEVICE_CODE_GRANT_TYPE),
                ],
                "poll_sign_in",
            )
            .await?;
        mapping::sign_in_poll(response)
    }
}
