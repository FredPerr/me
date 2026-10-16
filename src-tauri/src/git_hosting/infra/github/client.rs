//! HTTP client for GitHub's device flow and REST API.

use reqwest::header::ACCEPT;
use serde::de::DeserializeOwned;

use super::mapping::{status_error, RateLimitHeaders};
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::infra::http::{self, Answer};

const HOST_LABEL: &str = "github";
const JSON: &str = "application/json";
const GITHUB_JSON: &str = "application/vnd.github+json";
const RETRY_AFTER: &str = "retry-after";
const RATE_LIMIT_REMAINING: &str = "x-ratelimit-remaining";
const RATE_LIMIT_RESET: &str = "x-ratelimit-reset";
const LINK: &str = "link";

pub const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
pub const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub const API_BASE: &str = "https://api.github.com";
pub const USER_URL: &str = "https://api.github.com/user";

pub struct GithubClient {
    http: reqwest::Client,
}

impl GithubClient {
    pub fn new() -> Result<Self, GitHostingError> {
        Ok(Self {
            http: http::client()?,
        })
    }

    /// The device-flow endpoints take a form and answer JSON.
    pub async fn post_form<T: DeserializeOwned>(
        &self,
        url: &str,
        form: &[(&str, &str)],
        context: &str,
    ) -> Result<T, GitHostingError> {
        let request = self.http.post(url).header(ACCEPT, JSON).form(form);
        read(http::send(request).await?, context)
    }

    pub async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        token: &HostSecret,
        context: &str,
    ) -> Result<T, GitHostingError> {
        read(self.get(url, token).await?, context)
    }

    /// A page of a list and its `Link` header, which links to the next page.
    pub async fn get_page<T: DeserializeOwned>(
        &self,
        url: &str,
        token: &HostSecret,
        context: &str,
    ) -> Result<(T, Option<String>), GitHostingError> {
        let answer = self.get(url, token).await?;
        let link_header = answer.header(LINK).map(str::to_string);
        Ok((read(answer, context)?, link_header))
    }

    async fn get(&self, url: &str, token: &HostSecret) -> Result<Answer, GitHostingError> {
        let request = self
            .http
            .get(url)
            .bearer_auth(token.expose())
            .header(ACCEPT, GITHUB_JSON);
        http::send(request).await
    }
}

fn read<T: DeserializeOwned>(answer: Answer, context: &str) -> Result<T, GitHostingError> {
    let rate_limit = RateLimitHeaders {
        retry_after: answer.header(RETRY_AFTER),
        remaining: answer.header(RATE_LIMIT_REMAINING),
        reset: answer.header(RATE_LIMIT_RESET),
    };
    if let Some(error) = status_error(answer.status, rate_limit, http::now_epoch_seconds()) {
        answer.log_failure(HOST_LABEL, context);
        return Err(error);
    }
    http::decode(&answer, HOST_LABEL, context)
}
