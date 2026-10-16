use async_trait::async_trait;

use super::client::{BitbucketClient, API_BASE, USER_URL};
use super::wire::{CreatedPullRequestResponse, PullRequestPage, UserResponse};
use super::{mapping, new_pull_request_mapping, pull_request_mapping};
use crate::git_hosting::domain::capped::Capped;
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::hosted_account::HostedAccount;
use crate::git_hosting::domain::hosted_pull_request::HostedPullRequest;
use crate::git_hosting::domain::hosted_repository::HostedRepository;
use crate::git_hosting::domain::new_pull_request::{CreatedPullRequest, NewPullRequest};
use crate::git_hosting::domain::ports::GitHost;
use crate::git_hosting::infra::paging::{api_url, follow_pages, Page};

const OPEN_PULL_REQUESTS_QUERY: [(&str, &str); 3] = [
    ("state", "OPEN"),
    ("sort", "-updated_on"),
    ("pagelen", "50"),
];

pub struct BitbucketHost {
    client: BitbucketClient,
    username: String,
}

impl BitbucketHost {
    pub fn new(client: BitbucketClient, username: &str) -> Self {
        Self {
            client,
            username: username.to_string(),
        }
    }
}

#[async_trait]
impl GitHost for BitbucketHost {
    async fn current_account(&self) -> Result<HostedAccount, GitHostingError> {
        let user: UserResponse = self.client.get_json(USER_URL, "current_account").await?;
        Ok(mapping::account(user, &self.username))
    }

    async fn list_open_pull_requests(
        &self,
        repository: &HostedRepository,
    ) -> Result<Capped<HostedPullRequest>, GitHostingError> {
        let first_url = api_url(
            API_BASE,
            &[
                "repositories",
                &repository.owner,
                &repository.name,
                "pullrequests",
            ],
            &OPEN_PULL_REQUESTS_QUERY,
        )?;
        follow_pages(first_url, API_BASE, |url| async move {
            let page: PullRequestPage = self
                .client
                .get_json(&url, "list_open_pull_requests")
                .await?;
            Ok(Page {
                items: page
                    .values
                    .into_iter()
                    .map(pull_request_mapping::pull_request)
                    .collect(),
                next_url: page.next,
            })
        })
        .await
    }

    async fn find_branch_pull_request(
        &self,
        repository: &HostedRepository,
        branch: &str,
    ) -> Result<Option<HostedPullRequest>, GitHostingError> {
        let query = pull_request_mapping::branch_pull_requests_query(branch);
        let query: Vec<(&str, &str)> = query
            .iter()
            .map(|(name, value)| (*name, value.as_str()))
            .collect();
        let url = api_url(
            API_BASE,
            &[
                "repositories",
                &repository.owner,
                &repository.name,
                "pullrequests",
            ],
            &query,
        )?;
        let page: PullRequestPage = self
            .client
            .get_json(&url, "find_branch_pull_request")
            .await?;
        Ok(pull_request_mapping::branch_pull_request(page.values))
    }

    async fn create_pull_request(
        &self,
        repository: &HostedRepository,
        pull_request: &NewPullRequest,
    ) -> Result<CreatedPullRequest, GitHostingError> {
        let url = api_url(
            API_BASE,
            &[
                "repositories",
                &repository.owner,
                &repository.name,
                "pullrequests",
            ],
            &[],
        )?;
        let created: CreatedPullRequestResponse = self
            .client
            .post_json(
                &url,
                &new_pull_request_mapping::create_pull_request_request(pull_request),
                "create_pull_request",
            )
            .await?;
        new_pull_request_mapping::created_pull_request(created).inspect_err(|_| {
            eprintln!("[git_hosting] bitbucket create_pull_request answered without a web link");
        })
    }
}
