//! HTTP client for the Bitbucket Cloud REST API. API tokens authenticate REST
//! calls as a Bearer token; only `git` uses the `username:token` pair.

use reqwest::header::ACCEPT;
use serde::de::DeserializeOwned;

use super::mapping::status_error;
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::infra::http;

const HOST_LABEL: &str = "bitbucket";
const JSON: &str = "application/json";
const RETRY_AFTER: &str = "retry-after";

pub const API_BASE: &str = "https://api.bitbucket.org/2.0";
pub const USER_URL: &str = "https://api.bitbucket.org/2.0/user";

pub struct BitbucketClient {
    http: reqwest::Client,
    token: String,
}

impl BitbucketClient {
    pub fn new(token: &str) -> Result<Self, GitHostingError> {
        Ok(Self {
            http: http::client()?,
            token: token.to_string(),
        })
    }

    pub async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        context: &str,
    ) -> Result<T, GitHostingError> {
        let request = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .header(ACCEPT, JSON);
        let answer = http::send(request).await?;
        if let Some(error) = status_error(answer.status, answer.header(RETRY_AFTER)) {
            answer.log_failure(HOST_LABEL, context);
            return Err(error);
        }
        http::decode(&answer, HOST_LABEL, context)
    }
}
