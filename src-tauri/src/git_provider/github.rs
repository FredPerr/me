//! GitHub provider: OAuth Device Flow authentication and minimal API access.
//!
//! Device Flow is the right fit for a desktop app registered as a GitHub App:
//! no client secret and no redirect server are required. The user is shown a
//! short code to enter at <https://github.com/login/device>, and we poll for
//! the resulting access token.
//!
//! Docs: https://docs.github.com/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app#using-the-device-flow-to-generate-a-user-access-token

use serde::{Deserialize, Serialize};

use super::token_store;
use super::PROVIDER_GITHUB;

/// Client ID of the "me" GitHub App. This is public information (it ships in
/// every OAuth redirect), so embedding it is safe. There is no client secret
/// in the device flow.
const GITHUB_CLIENT_ID: &str = "Iv23lilfZx8UAX1Afhti";

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_API_USER_URL: &str = "https://api.github.com/user";
const GITHUB_API_REPOS_URL: &str = "https://api.github.com/user/repos";
const USER_AGENT: &str = "me-app";

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| error.to_string())
}

// --- Device flow: step 1, request a device + user code ----------------------

/// The verification details shown to the user to start authorization. Field
/// names are camelCased for the frontend port.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

/// Begin the device flow by requesting a user code from GitHub.
#[tauri::command]
pub async fn github_start_device_authorization() -> Result<DeviceAuthorization, String> {
    let response = http_client()?
        .post(DEVICE_CODE_URL)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[("client_id", GITHUB_CLIENT_ID)])
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!("GitHub returned status {}", response.status()));
    }

    let body: DeviceCodeResponse = response.json().await.map_err(|error| error.to_string())?;

    Ok(DeviceAuthorization {
        device_code: body.device_code,
        user_code: body.user_code,
        verification_uri: body.verification_uri,
        expires_in: body.expires_in,
        interval: body.interval,
    })
}

// --- Device flow: step 2, poll for the access token -------------------------

/// The outcome of a single poll for the access token. The frontend keeps
/// polling on `Pending`/`SlowDown` and stops on any other variant.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum DevicePollResult {
    /// The token was issued and stored. Authentication is complete.
    Authorized,
    /// The user has not yet entered the code. Keep polling.
    Pending,
    /// GitHub asked us to slow down; poll again after `interval` seconds.
    SlowDown { interval: u64 },
    /// The device code expired. The flow must be restarted.
    Expired,
    /// The user denied the authorization request.
    Denied,
}

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    interval: Option<u64>,
}

/// Poll GitHub once for the access token tied to `device_code`. On success the
/// token is written to the OS keychain and never returned to the frontend.
#[tauri::command]
pub async fn github_poll_for_access_token(device_code: String) -> Result<DevicePollResult, String> {
    let response = http_client()?
        .post(ACCESS_TOKEN_URL)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[
            ("client_id", GITHUB_CLIENT_ID),
            ("device_code", device_code.as_str()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .map_err(|error| error.to_string())?;

    let body: AccessTokenResponse = response.json().await.map_err(|error| error.to_string())?;

    if let Some(token) = body.access_token {
        token_store::save_token(PROVIDER_GITHUB, &token)?;
        return Ok(DevicePollResult::Authorized);
    }

    match body.error.as_deref() {
        Some("authorization_pending") => Ok(DevicePollResult::Pending),
        Some("slow_down") => Ok(DevicePollResult::SlowDown {
            interval: body.interval.unwrap_or(5),
        }),
        Some("expired_token") => Ok(DevicePollResult::Expired),
        Some("access_denied") => Ok(DevicePollResult::Denied),
        Some(other) => Err(other.to_string()),
        None => Err("Unexpected response from GitHub".to_string()),
    }
}

// --- Authenticated API access -----------------------------------------------

/// The authenticated GitHub user, as surfaced to the connected-account UI.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubUser {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GitHubUserResponse {
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

async fn require_token() -> Result<String, String> {
    token_store::read_token(PROVIDER_GITHUB)?.ok_or_else(|| "Not authenticated".to_string())
}

/// Fetch the currently authenticated user. Returns an error if no token is
/// stored (i.e. the account is not connected).
#[tauri::command]
pub async fn github_get_authenticated_user() -> Result<GitHubUser, String> {
    let token = require_token().await?;

    let response = http_client()?
        .get(GITHUB_API_USER_URL)
        .bearer_auth(&token)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!("GitHub returned status {}", response.status()));
    }

    let user: GitHubUserResponse = response.json().await.map_err(|error| error.to_string())?;

    Ok(GitHubUser {
        login: user.login,
        name: user.name,
        avatar_url: user.avatar_url,
    })
}

/// A repository the authenticated user has access to.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRepository {
    pub full_name: String,
    pub name: String,
    pub private: bool,
    pub clone_url: String,
    pub default_branch: String,
}

#[derive(Deserialize)]
struct GitHubRepositoryResponse {
    full_name: String,
    name: String,
    private: bool,
    clone_url: String,
    default_branch: String,
}

/// List repositories accessible to the authenticated user (first page, sorted
/// by most recently updated).
#[tauri::command]
pub async fn github_list_repositories() -> Result<Vec<GitHubRepository>, String> {
    let token = require_token().await?;

    let response = http_client()?
        .get(GITHUB_API_REPOS_URL)
        .bearer_auth(&token)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .query(&[("per_page", "100"), ("sort", "updated")])
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!("GitHub returned status {}", response.status()));
    }

    let repositories: Vec<GitHubRepositoryResponse> =
        response.json().await.map_err(|error| error.to_string())?;

    Ok(repositories
        .into_iter()
        .map(|repository| GitHubRepository {
            full_name: repository.full_name,
            name: repository.name,
            private: repository.private,
            clone_url: repository.clone_url,
            default_branch: repository.default_branch,
        })
        .collect())
}

// --- Pull requests ----------------------------------------------------------

/// An open pull request, as shown on the pull-requests page.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubPullRequest {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub author: Option<String>,
    pub head_branch: String,
    pub base_branch: String,
    pub draft: bool,
    pub updated_at: String,
}

#[derive(Deserialize)]
struct PullRequestResponse {
    number: u64,
    title: String,
    html_url: String,
    user: Option<PullRequestUser>,
    head: PullRequestRef,
    base: PullRequestRef,
    #[serde(default)]
    draft: bool,
    updated_at: String,
}

#[derive(Deserialize)]
struct PullRequestUser {
    login: String,
}

#[derive(Deserialize)]
struct PullRequestRef {
    #[serde(rename = "ref")]
    ref_name: String,
}

/// List open pull requests for `owner/repo`, most recently updated first.
#[tauri::command]
pub async fn github_list_pull_requests(
    owner: String,
    repo: String,
) -> Result<Vec<GitHubPullRequest>, String> {
    let token = require_token().await?;

    let url = format!("https://api.github.com/repos/{}/{}/pulls", owner, repo);
    let response = http_client()?
        .get(&url)
        .bearer_auth(&token)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .query(&[("state", "open"), ("sort", "updated"), ("direction", "desc")])
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!("GitHub returned status {}", response.status()));
    }

    let pull_requests: Vec<PullRequestResponse> =
        response.json().await.map_err(|error| error.to_string())?;

    Ok(pull_requests
        .into_iter()
        .map(|pull_request| GitHubPullRequest {
            number: pull_request.number,
            title: pull_request.title,
            html_url: pull_request.html_url,
            author: pull_request.user.map(|user| user.login),
            head_branch: pull_request.head.ref_name,
            base_branch: pull_request.base.ref_name,
            draft: pull_request.draft,
            updated_at: pull_request.updated_at,
        })
        .collect())
}

// --- Connection state --------------------------------------------------------

/// Whether an access token is currently stored for GitHub. Used on startup to
/// decide whether to show the connected or disconnected state without making a
/// network call.
#[tauri::command]
pub async fn github_is_connected() -> Result<bool, String> {
    Ok(token_store::read_token(PROVIDER_GITHUB)?.is_some())
}

/// Disconnect the GitHub account by deleting the stored access token.
#[tauri::command]
pub async fn github_disconnect() -> Result<(), String> {
    token_store::delete_token(PROVIDER_GITHUB)
}
