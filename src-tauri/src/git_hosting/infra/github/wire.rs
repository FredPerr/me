//! GitHub's JSON requests and answers, as sent. No `Debug` where a token can appear.

use serde::{Deserialize, Serialize};

/// `POST https://api.github.com/repos/{owner}/{name}/pulls`
#[derive(Debug, Serialize)]
pub struct CreatePullRequestRequest<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub head: &'a str,
    pub base: &'a str,
    pub draft: bool,
}

/// The answer to [`CreatePullRequestRequest`].
#[derive(Debug, Deserialize)]
pub struct CreatedPullRequestResponse {
    pub number: u32,
    pub html_url: String,
}

/// `POST https://github.com/login/device/code`
#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u32,
    pub interval: u32,
}

/// `POST https://github.com/login/oauth/access_token`: a token, or an error code.
#[derive(Deserialize)]
pub struct AccessTokenResponse {
    pub access_token: Option<String>,
    pub error: Option<String>,
    pub interval: Option<u32>,
}

/// `GET https://api.github.com/user`
#[derive(Debug, Deserialize)]
pub struct UserResponse {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

/// One item of `GET https://api.github.com/repos/{owner}/{name}/pulls`
#[derive(Debug, Deserialize)]
pub struct PullRequestResponse {
    pub number: u32,
    pub title: String,
    pub html_url: String,
    pub user: Option<PullRequestUser>,
    pub head: BranchReference,
    pub base: BranchReference,
    #[serde(default)]
    pub draft: bool,
    /// `open` or `closed`; a merged pull request is `closed` with `merged_at`.
    pub state: String,
    pub merged_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestUser {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct BranchReference {
    #[serde(rename = "ref")]
    pub name: String,
}
