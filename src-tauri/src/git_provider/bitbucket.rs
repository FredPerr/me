//! Bitbucket Cloud provider: API token authentication and minimal API access.
//!
//! Bitbucket has no OAuth Device Flow, so instead of a browser authorization
//! dance the user creates an API token (the replacement for the now-deprecated
//! App Passwords) and supplies it along with their Bitbucket username.
//!
//! Two auth styles are used depending on the channel:
//! - REST API calls here authenticate with `Authorization: Bearer <api_token>`,
//!   which API tokens support and which needs no accompanying identifier.
//! - `git` over HTTPS (in [`super::git_auth`]) uses HTTP Basic with
//!   `username:api_token`, since git has no bearer concept.
//!
//! The credential is stored in the OS keychain as a single `username:token`
//! string (never returned to the frontend). The REST client reads only the
//! token half; the `git` CLI injection reads the whole `username:token` pair.
//!
//! Docs:
//! - API tokens: https://support.atlassian.com/bitbucket-cloud/docs/using-api-tokens/
//! - REST API: https://developer.atlassian.com/cloud/bitbucket/rest/intro/

use serde::{Deserialize, Serialize};

use super::token_store;
use super::PROVIDER_BITBUCKET;

const BITBUCKET_API_BASE: &str = "https://api.bitbucket.org/2.0";
const USER_AGENT: &str = "me-app";

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| error.to_string())
}

/// Split a stored `username:token` credential into its two parts. The token may
/// itself contain colons, so we split only on the first one.
fn split_credential(credential: &str) -> Result<(String, String), String> {
    match credential.split_once(':') {
        Some((username, token)) if !username.is_empty() && !token.is_empty() => {
            Ok((username.to_string(), token.to_string()))
        }
        _ => Err("Stored Bitbucket credential is malformed".to_string()),
    }
}

/// Read the stored credential and return its (username, token) parts, or an
/// error when the account is not connected.
fn require_credential() -> Result<(String, String), String> {
    let credential = token_store::read_token(PROVIDER_BITBUCKET)?
        .ok_or_else(|| "Not authenticated".to_string())?;
    split_credential(&credential)
}

/// Read just the API token for REST calls (which use Bearer auth), or an error
/// when the account is not connected.
fn require_token() -> Result<String, String> {
    let (_username, token) = require_credential()?;
    Ok(token)
}

/// Build an error string for a non-success Bitbucket response that includes the
/// response body. Bitbucket returns a JSON payload (e.g.
/// `{"type":"error","error":{"message":"..."}}`) that is essential for
/// diagnosing failures. Consumes the response because the body can only be read
/// once.
async fn describe_error_response(context: &str, response: reqwest::Response) -> String {
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|error| format!("<failed to read body: {}>", error));
    eprintln!(
        "[bitbucket] {} failed: status {} body {}",
        context, status, body
    );
    format!(
        "Bitbucket returned status {} for {}: {}",
        status, context, body
    )
}

// --- Authenticated user -----------------------------------------------------

/// The authenticated Bitbucket user, surfaced to the connected-account UI.
/// Field names are camelCased for the frontend port.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BitbucketUser {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct BitbucketUserResponse {
    /// Immutable account handle used as the Basic-auth username.
    username: Option<String>,
    /// Human-friendly display name.
    display_name: Option<String>,
    links: Option<UserLinks>,
}

#[derive(Deserialize)]
struct UserLinks {
    avatar: Option<Link>,
}

#[derive(Deserialize)]
struct Link {
    href: Option<String>,
}

/// Fetch the authenticated user for the given credentials. Shared by the
/// connect-time validation and the connected-account UI.
async fn fetch_user(username: &str, token: &str) -> Result<BitbucketUser, String> {
    let url = format!("{}/user", BITBUCKET_API_BASE);
    let response = http_client()?
        .get(&url)
        .bearer_auth(token)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| {
            eprintln!("[bitbucket] get_authenticated_user request error: {}", error);
            error.to_string()
        })?;

    if !response.status().is_success() {
        return Err(describe_error_response("get_authenticated_user", response).await);
    }

    let user: BitbucketUserResponse = response.json().await.map_err(|error| {
        eprintln!("[bitbucket] get_authenticated_user decode error: {}", error);
        error.to_string()
    })?;

    Ok(BitbucketUser {
        login: user.username.unwrap_or_else(|| username.to_string()),
        name: user.display_name,
        avatar_url: user.links.and_then(|links| links.avatar).and_then(|a| a.href),
    })
}

/// Validate `username` + `token` by fetching the authenticated user, then store
/// the credential in the OS keychain. The credential is never returned to the
/// frontend. Rejects (without storing) when the credentials are invalid.
#[tauri::command]
pub async fn bitbucket_connect_with_token(
    username: String,
    token: String,
) -> Result<(), String> {
    let username = username.trim().to_string();
    let token = token.trim().to_string();
    if username.is_empty() || token.is_empty() {
        return Err("Username and API token are required".to_string());
    }

    // Validate before persisting so a bad credential never lingers in the keychain.
    fetch_user(&username, &token).await?;

    let credential = format!("{}:{}", username, token);
    token_store::save_token(PROVIDER_BITBUCKET, &credential)
}

/// Fetch the currently authenticated user. Returns an error if no credential is
/// stored (i.e. the account is not connected).
#[tauri::command]
pub async fn bitbucket_get_authenticated_user() -> Result<BitbucketUser, String> {
    let (username, token) = require_credential()?;
    fetch_user(&username, &token).await
}

// --- Repositories -----------------------------------------------------------

/// A repository the authenticated user has access to.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BitbucketRepository {
    pub full_name: String,
    pub name: String,
    pub private: bool,
    pub clone_url: String,
    pub default_branch: String,
}

#[derive(Deserialize)]
struct RepositoryListResponse {
    #[serde(default)]
    values: Vec<RepositoryResponse>,
}

#[derive(Deserialize)]
struct RepositoryResponse {
    full_name: String,
    name: String,
    #[serde(default)]
    is_private: bool,
    links: Option<RepositoryLinks>,
    mainbranch: Option<MainBranch>,
}

#[derive(Deserialize)]
struct RepositoryLinks {
    #[serde(default)]
    clone: Vec<CloneLink>,
}

#[derive(Deserialize)]
struct CloneLink {
    name: String,
    href: String,
}

#[derive(Deserialize)]
struct MainBranch {
    name: String,
}

/// List repositories accessible to the authenticated user, most recently
/// updated first (first page).
#[tauri::command]
pub async fn bitbucket_list_repositories() -> Result<Vec<BitbucketRepository>, String> {
    let token = require_token()?;

    let url = format!("{}/repositories", BITBUCKET_API_BASE);
    let response = http_client()?
        .get(&url)
        .bearer_auth(&token)
        .header(reqwest::header::ACCEPT, "application/json")
        .query(&[("role", "member"), ("sort", "-updated_on"), ("pagelen", "100")])
        .send()
        .await
        .map_err(|error| {
            eprintln!("[bitbucket] list_repositories request error: {}", error);
            error.to_string()
        })?;

    if !response.status().is_success() {
        return Err(describe_error_response("list_repositories", response).await);
    }

    let body: RepositoryListResponse = response.json().await.map_err(|error| {
        eprintln!("[bitbucket] list_repositories decode error: {}", error);
        error.to_string()
    })?;

    Ok(body
        .values
        .into_iter()
        .map(|repository| {
            let clone_url = repository
                .links
                .map(|links| links.clone)
                .and_then(|clones| {
                    clones
                        .into_iter()
                        .find(|link| link.name == "https")
                        .map(|link| link.href)
                })
                .unwrap_or_default();
            BitbucketRepository {
                full_name: repository.full_name,
                name: repository.name,
                private: repository.is_private,
                clone_url,
                default_branch: repository
                    .mainbranch
                    .map(|branch| branch.name)
                    .unwrap_or_else(|| "main".to_string()),
            }
        })
        .collect())
}

// --- Pull requests ----------------------------------------------------------

/// An open pull request, as shown on the pull-requests page.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BitbucketPullRequest {
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
struct PullRequestListResponse {
    #[serde(default)]
    values: Vec<PullRequestResponse>,
}

#[derive(Deserialize)]
struct PullRequestResponse {
    id: u64,
    title: String,
    author: Option<PullRequestAuthor>,
    source: PullRequestEndpoint,
    destination: PullRequestEndpoint,
    #[serde(default)]
    updated_on: String,
    #[serde(default)]
    draft: bool,
    links: Option<PullRequestLinks>,
}

#[derive(Deserialize)]
struct PullRequestAuthor {
    #[serde(default)]
    nickname: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct PullRequestEndpoint {
    branch: PullRequestBranch,
}

#[derive(Deserialize)]
struct PullRequestBranch {
    name: String,
}

#[derive(Deserialize)]
struct PullRequestLinks {
    html: Option<Link>,
}

/// List open pull requests for `workspace/repo_slug`, most recently updated
/// first.
#[tauri::command]
pub async fn bitbucket_list_pull_requests(
    workspace: String,
    repo_slug: String,
) -> Result<Vec<BitbucketPullRequest>, String> {
    let token = require_token()?;

    let url = format!(
        "{}/repositories/{}/{}/pullrequests",
        BITBUCKET_API_BASE, workspace, repo_slug
    );
    eprintln!(
        "[bitbucket] list_pull_requests {}/{} -> {}",
        workspace, repo_slug, url
    );
    let response = http_client()?
        .get(&url)
        .bearer_auth(&token)
        .header(reqwest::header::ACCEPT, "application/json")
        .query(&[("state", "OPEN"), ("sort", "-updated_on"), ("pagelen", "50")])
        .send()
        .await
        .map_err(|error| {
            eprintln!(
                "[bitbucket] list_pull_requests {}/{} request error: {}",
                workspace, repo_slug, error
            );
            error.to_string()
        })?;

    if !response.status().is_success() {
        let context = format!("list_pull_requests {}/{}", workspace, repo_slug);
        return Err(describe_error_response(&context, response).await);
    }

    let body: PullRequestListResponse = response.json().await.map_err(|error| {
        eprintln!(
            "[bitbucket] list_pull_requests {}/{} decode error: {}",
            workspace, repo_slug, error
        );
        error.to_string()
    })?;

    eprintln!(
        "[bitbucket] list_pull_requests {}/{} returned {} open PR(s)",
        workspace,
        repo_slug,
        body.values.len()
    );

    Ok(body
        .values
        .into_iter()
        .map(|pull_request| BitbucketPullRequest {
            number: pull_request.id,
            title: pull_request.title,
            html_url: pull_request
                .links
                .and_then(|links| links.html)
                .and_then(|link| link.href)
                .unwrap_or_default(),
            author: pull_request.author.and_then(|author| {
                author.nickname.or(author.display_name)
            }),
            head_branch: pull_request.source.branch.name,
            base_branch: pull_request.destination.branch.name,
            draft: pull_request.draft,
            updated_at: pull_request.updated_on,
        })
        .collect())
}

// --- Create pull request ----------------------------------------------------

/// The newly created pull request, surfaced to the UI so it can link to it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedPullRequest {
    pub number: u64,
    pub html_url: String,
}

#[derive(Deserialize)]
struct CreatedPullRequestResponse {
    id: u64,
    links: Option<PullRequestLinks>,
}

#[derive(Serialize)]
struct CreatePullRequestBody<'a> {
    title: &'a str,
    description: &'a str,
    source: BranchEndpoint<'a>,
    destination: BranchEndpoint<'a>,
    close_source_branch: bool,
    draft: bool,
}

#[derive(Serialize)]
struct BranchEndpoint<'a> {
    branch: BranchName<'a>,
}

#[derive(Serialize)]
struct BranchName<'a> {
    name: &'a str,
}

/// Open a pull request on `workspace/repo_slug` from `head` into `base`. The
/// `head` branch must already be pushed to the remote. Mirrors the Bitbucket
/// REST "Create a pull request" endpoint, including the `draft` flag.
///
/// Docs: https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pullrequests/#api-repositories-workspace-repo-slug-pullrequests-post
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn bitbucket_create_pull_request(
    workspace: String,
    repo_slug: String,
    title: String,
    body: String,
    head: String,
    base: String,
    draft: bool,
) -> Result<CreatedPullRequest, String> {
    let token = require_token()?;

    let url = format!(
        "{}/repositories/{}/{}/pullrequests",
        BITBUCKET_API_BASE, workspace, repo_slug
    );
    eprintln!(
        "[bitbucket] create_pull_request {}/{} {} -> {}",
        workspace, repo_slug, head, base
    );
    let response = http_client()?
        .post(&url)
        .bearer_auth(&token)
        .header(reqwest::header::ACCEPT, "application/json")
        .json(&CreatePullRequestBody {
            title: &title,
            description: &body,
            source: BranchEndpoint {
                branch: BranchName { name: &head },
            },
            destination: BranchEndpoint {
                branch: BranchName { name: &base },
            },
            close_source_branch: false,
            draft,
        })
        .send()
        .await
        .map_err(|error| {
            eprintln!(
                "[bitbucket] create_pull_request {}/{} request error: {}",
                workspace, repo_slug, error
            );
            error.to_string()
        })?;

    if !response.status().is_success() {
        let context = format!("create_pull_request {}/{}", workspace, repo_slug);
        return Err(describe_error_response(&context, response).await);
    }

    let created: CreatedPullRequestResponse = response.json().await.map_err(|error| {
        eprintln!(
            "[bitbucket] create_pull_request {}/{} decode error: {}",
            workspace, repo_slug, error
        );
        error.to_string()
    })?;

    eprintln!(
        "[bitbucket] create_pull_request {}/{} created PR #{}",
        workspace, repo_slug, created.id
    );

    Ok(CreatedPullRequest {
        number: created.id,
        html_url: created
            .links
            .and_then(|links| links.html)
            .and_then(|link| link.href)
            .unwrap_or_default(),
    })
}

// --- Connection state --------------------------------------------------------

/// Whether a credential is currently stored for Bitbucket. Used on startup to
/// decide whether to show the connected or disconnected state without making a
/// network call.
#[tauri::command]
pub async fn bitbucket_is_connected() -> Result<bool, String> {
    Ok(token_store::read_token(PROVIDER_BITBUCKET)?.is_some())
}

/// Disconnect the Bitbucket account by deleting the stored credential.
#[tauri::command]
pub async fn bitbucket_disconnect() -> Result<(), String> {
    token_store::delete_token(PROVIDER_BITBUCKET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_credential_into_username_and_token() {
        let (username, token) = split_credential("alice:app-pw").unwrap();
        assert_eq!(username, "alice");
        assert_eq!(token, "app-pw");
    }

    #[test]
    fn splits_only_on_the_first_colon() {
        let (username, token) = split_credential("alice:tok:en:with:colons").unwrap();
        assert_eq!(username, "alice");
        assert_eq!(token, "tok:en:with:colons");
    }

    #[test]
    fn rejects_credential_without_a_colon() {
        assert!(split_credential("no-colon").is_err());
    }

    #[test]
    fn rejects_credential_with_empty_parts() {
        assert!(split_credential(":token").is_err());
        assert!(split_credential("username:").is_err());
    }
}
