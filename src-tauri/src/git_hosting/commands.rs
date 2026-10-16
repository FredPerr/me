//! Typed Tauri commands of the Git Hosting context and their wire types.
//! Secrets never leave Rust: sign-in commands only return what the UI shows.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::application::service::GitHostingService;
use super::domain::error::{GitHostingError, InputField, PushFailureReason};
use super::domain::host_kind::HostKind;
use super::domain::hosted_account::{HostedAccount, SignedInHost};
use super::domain::hosted_pull_request::{HostedPullRequest, PullRequestState};
use super::domain::new_pull_request::{CreatedPullRequest, NewPullRequest};
use super::domain::project_pull_requests::{
    BranchPullRequest, BranchPullRequestStatus, PullRequestsStatus, RepositoryPullRequests,
};
use super::domain::sign_in::{BrowserSignInStep, SignInProgress};
use super::GitHosting;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum HostKindView {
    Github,
    Bitbucket,
}

impl From<HostKind> for HostKindView {
    fn from(host: HostKind) -> Self {
        match host {
            HostKind::Github => Self::Github,
            HostKind::Bitbucket => Self::Bitbucket,
        }
    }
}

impl From<HostKindView> for HostKind {
    fn from(host: HostKindView) -> Self {
        match host {
            HostKindView::Github => Self::Github,
            HostKindView::Bitbucket => Self::Bitbucket,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum GitHostingErrorKind {
    NotConnected,
    InvalidInput,
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited,
    Network,
    Timeout,
    ProviderError,
    InvalidResponse,
    Storage,
    NoRemote,
    UnsupportedHost,
    PushSkipped,
    PushFailed,
}

/// A failed Git Hosting command on the wire. `message` is a fixed English
/// sentence per kind and never holds a response body or a secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GitHostingCommandError {
    pub kind: GitHostingErrorKind,
    pub message: String,
    pub retry_after_seconds: Option<u32>,
    pub status: Option<u16>,
}

impl GitHostingCommandError {
    fn new(kind: GitHostingErrorKind, message: &str) -> Self {
        Self {
            kind,
            message: message.to_string(),
            retry_after_seconds: None,
            status: None,
        }
    }
}

fn invalid_input_message(field: InputField) -> &'static str {
    match field {
        InputField::Host => "This Git host does not support this way of signing in.",
        InputField::Username => "Enter a username.",
        InputField::Token => "Enter a token.",
        InputField::DeviceCode => "The sign-in code is missing or unknown. Start again.",
        InputField::Title => "Enter a title.",
        InputField::BaseBranch => "Choose a base branch other than the branch to merge.",
    }
}

fn push_failed_message(reason: PushFailureReason) -> &'static str {
    match reason {
        PushFailureReason::Rejected => {
            "The branch could not be pushed: the remote branch has changes it does not have. Pull them first."
        }
        PushFailureReason::Unauthorized => {
            "The branch could not be pushed: the remote rejected the credentials."
        }
        PushFailureReason::Network => "The branch could not be pushed: the remote could not be reached.",
        PushFailureReason::Other => "The branch could not be pushed.",
    }
}

impl From<GitHostingError> for GitHostingCommandError {
    fn from(error: GitHostingError) -> Self {
        use GitHostingErrorKind as Kind;
        match error {
            GitHostingError::NotConnected => Self::new(
                Kind::NotConnected,
                "You are not signed in to this Git host.",
            ),
            GitHostingError::InvalidInput { field } => {
                Self::new(Kind::InvalidInput, invalid_input_message(field))
            }
            GitHostingError::Unauthorized => {
                Self::new(Kind::Unauthorized, "The Git host rejected the credentials.")
            }
            GitHostingError::Forbidden => Self::new(
                Kind::Forbidden,
                "The Git host denied access to this resource.",
            ),
            GitHostingError::NotFound => {
                Self::new(Kind::NotFound, "The Git host could not find this resource.")
            }
            GitHostingError::RateLimited {
                retry_after_seconds,
            } => Self {
                retry_after_seconds,
                ..Self::new(Kind::RateLimited, "The Git host rate limit was reached.")
            },
            GitHostingError::Network => {
                Self::new(Kind::Network, "The Git host could not be reached.")
            }
            GitHostingError::Timeout => {
                Self::new(Kind::Timeout, "The request to the Git host timed out.")
            }
            GitHostingError::ProviderError { status } => Self {
                status,
                ..Self::new(
                    Kind::ProviderError,
                    "The Git host returned an unexpected error.",
                )
            },
            GitHostingError::InvalidResponse => Self::new(
                Kind::InvalidResponse,
                "The Git host returned an answer that could not be read.",
            ),
            GitHostingError::Storage => {
                Self::new(Kind::Storage, "Local storage could not be accessed.")
            }
            GitHostingError::NoRemote => {
                Self::new(Kind::NoRemote, "This repository has no remote.")
            }
            GitHostingError::UnsupportedHost => Self::new(
                Kind::UnsupportedHost,
                "This repository's remote is not on a supported Git host.",
            ),
            GitHostingError::PushSkipped => Self::new(
                Kind::PushSkipped,
                "The branch could not be pushed: it has no remote branch to push to.",
            ),
            GitHostingError::PushFailed { reason } => {
                Self::new(Kind::PushFailed, push_failed_message(reason))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HostedAccountView {
    pub host: HostKindView,
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

impl From<HostedAccount> for HostedAccountView {
    fn from(account: HostedAccount) -> Self {
        Self {
            host: account.host.into(),
            login: account.login,
            name: account.name,
            avatar_url: account.avatar_url,
        }
    }
}

/// A Git host with a stored secret. `account` is null when it could not be
/// fetched yet, e.g. while offline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SignedInHostView {
    pub host: HostKindView,
    pub account: Option<HostedAccountView>,
}

impl From<SignedInHost> for SignedInHostView {
    fn from(signed_in: SignedInHost) -> Self {
        Self {
            host: signed_in.host.into(),
            account: signed_in.account.map(HostedAccountView::from),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSignInStepView {
    pub user_code: String,
    pub verification_url: String,
    pub device_code: String,
    pub interval_seconds: u32,
    pub expires_in_seconds: u32,
}

impl From<BrowserSignInStep> for BrowserSignInStepView {
    fn from(step: BrowserSignInStep) -> Self {
        Self {
            user_code: step.user_code,
            verification_url: step.verification_url,
            device_code: step.device_code,
            interval_seconds: step.interval_seconds,
            expires_in_seconds: step.expires_in_seconds,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum SignInProgressView {
    Authorized,
    Pending,
    SlowDown {
        #[serde(rename = "intervalSeconds")]
        interval_seconds: u32,
    },
    Expired,
    Denied,
}

impl From<SignInProgress> for SignInProgressView {
    fn from(progress: SignInProgress) -> Self {
        match progress {
            SignInProgress::Authorized => Self::Authorized,
            SignInProgress::Pending => Self::Pending,
            SignInProgress::SlowDown { interval_seconds } => Self::SlowDown { interval_seconds },
            SignInProgress::Expired => Self::Expired,
            SignInProgress::Denied => Self::Denied,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PullRequestStateView {
    Open,
    Merged,
    Closed,
}

impl From<PullRequestState> for PullRequestStateView {
    fn from(state: PullRequestState) -> Self {
        match state {
            PullRequestState::Open => Self::Open,
            PullRequestState::Merged => Self::Merged,
            PullRequestState::Closed => Self::Closed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HostedPullRequestView {
    pub number: u32,
    pub title: String,
    pub url: String,
    pub author: Option<String>,
    pub head_branch: String,
    pub base_branch: String,
    pub draft: bool,
    pub state: PullRequestStateView,
    pub updated_at: String,
}

impl From<HostedPullRequest> for HostedPullRequestView {
    fn from(pull_request: HostedPullRequest) -> Self {
        Self {
            number: pull_request.number,
            title: pull_request.title,
            url: pull_request.url,
            author: pull_request.author,
            head_branch: pull_request.head_branch,
            base_branch: pull_request.base_branch,
            draft: pull_request.draft,
            state: pull_request.state.into(),
            updated_at: pull_request.updated_at,
        }
    }
}

/// What became of loading one repository's open pull requests. `truncated`
/// says the list was cut short after the page limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PullRequestsStatusView {
    NoRemote,
    UnsupportedHost,
    NotConnected,
    Failed {
        error: GitHostingCommandError,
    },
    Ok {
        #[serde(rename = "pullRequests")]
        pull_requests: Vec<HostedPullRequestView>,
        truncated: bool,
    },
}

impl From<PullRequestsStatus> for PullRequestsStatusView {
    fn from(status: PullRequestsStatus) -> Self {
        match status {
            PullRequestsStatus::NoRemote => Self::NoRemote,
            PullRequestsStatus::UnsupportedHost => Self::UnsupportedHost,
            PullRequestsStatus::NotConnected => Self::NotConnected,
            PullRequestsStatus::Failed(error) => Self::Failed {
                error: error.into(),
            },
            PullRequestsStatus::Ok(capped) => Self::Ok {
                pull_requests: capped.items.into_iter().map(Into::into).collect(),
                truncated: capped.truncated,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryPullRequestsView {
    pub repository_id: String,
    pub repository_name: String,
    pub host: Option<HostKindView>,
    pub status: PullRequestsStatusView,
}

impl From<RepositoryPullRequests> for RepositoryPullRequestsView {
    fn from(repository: RepositoryPullRequests) -> Self {
        Self {
            repository_id: repository.repository_id,
            repository_name: repository.repository_name,
            host: repository.host.map(HostKindView::from),
            status: repository.status.into(),
        }
    }
}

/// What became of looking up the pull request of one context branch.
/// `pullRequest` is null when the branch has none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BranchPullRequestStatusView {
    NoRemote,
    UnsupportedHost,
    NotConnected,
    Failed {
        error: GitHostingCommandError,
    },
    Ok {
        #[serde(rename = "pullRequest")]
        pull_request: Option<HostedPullRequestView>,
    },
}

impl From<BranchPullRequestStatus> for BranchPullRequestStatusView {
    fn from(status: BranchPullRequestStatus) -> Self {
        match status {
            PullRequestsStatus::NoRemote => Self::NoRemote,
            PullRequestsStatus::UnsupportedHost => Self::UnsupportedHost,
            PullRequestsStatus::NotConnected => Self::NotConnected,
            PullRequestsStatus::Failed(error) => Self::Failed {
                error: error.into(),
            },
            PullRequestsStatus::Ok(pull_request) => Self::Ok {
                pull_request: pull_request.map(Into::into),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BranchPullRequestView {
    pub repository_id: String,
    pub branch: String,
    pub host: Option<HostKindView>,
    pub status: BranchPullRequestStatusView,
}

impl From<BranchPullRequest> for BranchPullRequestView {
    fn from(found: BranchPullRequest) -> Self {
        Self {
            repository_id: found.repository_id,
            branch: found.branch,
            host: found.host.map(HostKindView::from),
            status: found.status.into(),
        }
    }
}

/// A pull request to open from the context branch `head` into `base`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewPullRequestInput {
    pub title: String,
    pub body: String,
    pub head: String,
    pub base: String,
    pub draft: bool,
}

impl From<NewPullRequestInput> for NewPullRequest {
    fn from(input: NewPullRequestInput) -> Self {
        Self {
            title: input.title,
            body: input.body,
            head: input.head,
            base: input.base,
            draft: input.draft,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatedPullRequestView {
    pub number: u32,
    pub url: String,
}

impl From<CreatedPullRequest> for CreatedPullRequestView {
    fn from(created: CreatedPullRequest) -> Self {
        Self {
            number: created.number,
            url: created.url,
        }
    }
}

fn service(git_hosting: &State<'_, GitHosting>) -> Arc<GitHostingService> {
    git_hosting.service.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn git_hosting_list_accounts(
    git_hosting: State<'_, GitHosting>,
) -> Result<Vec<SignedInHostView>, GitHostingCommandError> {
    let signed_in_hosts = service(&git_hosting).list_accounts().await?;
    Ok(signed_in_hosts
        .into_iter()
        .map(SignedInHostView::from)
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn git_hosting_start_sign_in(
    git_hosting: State<'_, GitHosting>,
    host: HostKindView,
) -> Result<BrowserSignInStepView, GitHostingCommandError> {
    let step = service(&git_hosting)
        .start_browser_sign_in(host.into())
        .await?;
    Ok(step.into())
}

#[tauri::command]
#[specta::specta]
pub async fn git_hosting_poll_sign_in(
    git_hosting: State<'_, GitHosting>,
    host: HostKindView,
    device_code: String,
) -> Result<SignInProgressView, GitHostingCommandError> {
    let progress = service(&git_hosting)
        .poll_browser_sign_in(host.into(), &device_code)
        .await?;
    Ok(progress.into())
}

#[tauri::command]
#[specta::specta]
pub async fn git_hosting_sign_in_with_token(
    git_hosting: State<'_, GitHosting>,
    host: HostKindView,
    username: String,
    token: String,
) -> Result<HostedAccountView, GitHostingCommandError> {
    let account = service(&git_hosting)
        .sign_in_with_token(host.into(), &username, &token)
        .await?;
    Ok(account.into())
}

#[tauri::command]
#[specta::specta]
pub async fn git_hosting_sign_out(
    git_hosting: State<'_, GitHosting>,
    host: HostKindView,
) -> Result<(), GitHostingCommandError> {
    service(&git_hosting).sign_out(host.into()).await?;
    Ok(())
}

/// The open pull requests of each repository of the Project, in its order.
#[tauri::command]
#[specta::specta]
pub async fn git_hosting_project_pull_requests(
    git_hosting: State<'_, GitHosting>,
    project_tag: String,
) -> Result<Vec<RepositoryPullRequestsView>, GitHostingCommandError> {
    let repositories = service(&git_hosting)
        .project_open_pull_requests(&project_tag)
        .await?;
    Ok(repositories
        .into_iter()
        .map(RepositoryPullRequestsView::from)
        .collect())
}

/// The pull request of each branch the Project's contexts use, once per
/// repository and branch: one call for the whole board.
#[tauri::command]
#[specta::specta]
pub async fn git_hosting_context_pull_requests(
    git_hosting: State<'_, GitHosting>,
    project_tag: String,
) -> Result<Vec<BranchPullRequestView>, GitHostingCommandError> {
    let found = service(&git_hosting)
        .context_branch_pull_requests(&project_tag)
        .await?;
    Ok(found.into_iter().map(BranchPullRequestView::from).collect())
}

/// Pushes the context branch `input.head` of the repository, then opens its
/// pull request on the Git host of the repository's remote.
#[tauri::command]
#[specta::specta]
pub async fn git_hosting_create_pull_request(
    git_hosting: State<'_, GitHosting>,
    project_tag: String,
    repository_id: String,
    input: NewPullRequestInput,
) -> Result<CreatedPullRequestView, GitHostingCommandError> {
    let created = service(&git_hosting)
        .create_pull_request(&project_tag, &repository_id, input.into())
        .await?;
    Ok(created.into())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::git_hosting::domain::capped::Capped;

    const RETRY_AFTER_SECONDS: u32 = 30;
    const SERVER_ERROR_STATUS: u16 = 502;
    const INTERVAL_SECONDS: u32 = 10;

    fn wire(error: GitHostingError) -> serde_json::Value {
        serde_json::to_value(GitHostingCommandError::from(error)).unwrap()
    }

    #[test]
    fn an_unauthorized_error_says_the_credentials_were_rejected() {
        assert_eq!(
            wire(GitHostingError::Unauthorized),
            json!({
                "kind": "unauthorized",
                "message": "The Git host rejected the credentials.",
                "retryAfterSeconds": null,
                "status": null,
            })
        );
    }

    #[test]
    fn a_rate_limit_carries_the_retry_time() {
        let error = wire(GitHostingError::RateLimited {
            retry_after_seconds: Some(RETRY_AFTER_SECONDS),
        });

        assert_eq!(error["kind"], "rateLimited");
        assert_eq!(error["retryAfterSeconds"], RETRY_AFTER_SECONDS);
    }

    #[test]
    fn a_provider_error_carries_the_status() {
        let error = wire(GitHostingError::ProviderError {
            status: Some(SERVER_ERROR_STATUS),
        });

        assert_eq!(error["kind"], "providerError");
        assert_eq!(error["status"], SERVER_ERROR_STATUS);
    }

    #[test]
    fn invalid_input_names_the_missing_field() {
        let error = wire(GitHostingError::invalid_input(InputField::Token));

        assert_eq!(error["kind"], "invalidInput");
        assert_eq!(error["message"], "Enter a token.");
    }

    #[test]
    fn a_failed_push_says_why_without_git_output() {
        let error = wire(GitHostingError::PushFailed {
            reason: PushFailureReason::Rejected,
        });

        assert_eq!(error["kind"], "pushFailed");
        assert_eq!(
            error["message"],
            "The branch could not be pushed: the remote branch has changes it does not have. Pull them first."
        );
    }

    #[test]
    fn repositories_without_a_usable_remote_are_sent_by_kind() {
        for (error, kind) in [
            (GitHostingError::NoRemote, "noRemote"),
            (GitHostingError::UnsupportedHost, "unsupportedHost"),
            (GitHostingError::PushSkipped, "pushSkipped"),
        ] {
            assert_eq!(wire(error)["kind"], kind);
        }
    }

    #[test]
    fn a_new_pull_request_is_read_in_camel_case() {
        const TITLE: &str = "Add the login page";
        const BODY: &str = "Adds a login page.";
        const HEAD: &str = "feature/login";
        const BASE: &str = "main";

        let input: NewPullRequestInput = serde_json::from_value(json!({
            "title": TITLE,
            "body": BODY,
            "head": HEAD,
            "base": BASE,
            "draft": true,
        }))
        .unwrap();

        assert_eq!(
            NewPullRequest::from(input),
            NewPullRequest {
                title: TITLE.to_string(),
                body: BODY.to_string(),
                head: HEAD.to_string(),
                base: BASE.to_string(),
                draft: true,
            }
        );
    }

    #[test]
    fn a_created_pull_request_is_sent_with_its_number_and_link() {
        const NUMBER: u32 = 7;
        const URL: &str = "https://github.com/acme/api/pull/7";

        let created = CreatedPullRequestView::from(CreatedPullRequest {
            number: NUMBER,
            url: URL.to_string(),
        });

        assert_eq!(
            serde_json::to_value(created).unwrap(),
            json!({ "number": NUMBER, "url": URL })
        );
    }

    #[test]
    fn hosts_are_sent_by_identifier() {
        for host in HostKind::ALL {
            assert_eq!(
                serde_json::to_value(HostKindView::from(host)).unwrap(),
                json!(host.as_str())
            );
        }
    }

    mod pull_requests {
        use super::*;

        const NUMBER: u32 = 7;
        const TITLE: &str = "Add the login page";
        const URL: &str = "https://github.com/acme/api/pull/7";
        const AUTHOR: &str = "octocat";
        const HEAD_BRANCH: &str = "feature/login";
        const BASE_BRANCH: &str = "main";
        const UPDATED_AT: &str = "2024-05-01T12:00:00Z";
        const REPOSITORY_ID: &str = "api-id";
        const REPOSITORY_NAME: &str = "api";

        fn merged_pull_request() -> HostedPullRequest {
            HostedPullRequest {
                number: NUMBER,
                title: TITLE.to_string(),
                url: URL.to_string(),
                author: Some(AUTHOR.to_string()),
                head_branch: HEAD_BRANCH.to_string(),
                base_branch: BASE_BRANCH.to_string(),
                draft: false,
                state: PullRequestState::Merged,
                updated_at: UPDATED_AT.to_string(),
            }
        }

        fn status_wire(status: PullRequestsStatus) -> serde_json::Value {
            serde_json::to_value(PullRequestsStatusView::from(status)).unwrap()
        }

        #[test]
        fn a_listed_repository_is_sent_with_its_pull_requests() {
            let repository = RepositoryPullRequestsView::from(RepositoryPullRequests {
                repository_id: REPOSITORY_ID.to_string(),
                repository_name: REPOSITORY_NAME.to_string(),
                host: Some(HostKind::Github),
                status: PullRequestsStatus::Ok(Capped {
                    items: vec![merged_pull_request()],
                    truncated: true,
                }),
            });

            assert_eq!(
                serde_json::to_value(repository).unwrap(),
                json!({
                    "repositoryId": REPOSITORY_ID,
                    "repositoryName": REPOSITORY_NAME,
                    "host": "github",
                    "status": {
                        "kind": "ok",
                        "truncated": true,
                        "pullRequests": [{
                            "number": NUMBER,
                            "title": TITLE,
                            "url": URL,
                            "author": AUTHOR,
                            "headBranch": HEAD_BRANCH,
                            "baseBranch": BASE_BRANCH,
                            "draft": false,
                            "state": "merged",
                            "updatedAt": UPDATED_AT,
                        }],
                    },
                })
            );
        }

        #[test]
        fn a_found_branch_pull_request_is_sent_with_it() {
            let found = BranchPullRequestView::from(BranchPullRequest {
                repository_id: REPOSITORY_ID.to_string(),
                branch: HEAD_BRANCH.to_string(),
                host: Some(HostKind::Bitbucket),
                status: PullRequestsStatus::Ok(Some(merged_pull_request())),
            });

            let wire = serde_json::to_value(found).unwrap();

            assert_eq!(wire["repositoryId"], REPOSITORY_ID);
            assert_eq!(wire["branch"], HEAD_BRANCH);
            assert_eq!(wire["host"], "bitbucket");
            assert_eq!(wire["status"]["kind"], "ok");
            assert_eq!(wire["status"]["pullRequest"]["number"], NUMBER);
            assert_eq!(wire["status"]["pullRequest"]["state"], "merged");
        }

        #[test]
        fn a_branch_without_pull_request_is_sent_with_null() {
            let status = BranchPullRequestStatusView::from(PullRequestsStatus::Ok(None));

            assert_eq!(
                serde_json::to_value(status).unwrap(),
                json!({ "kind": "ok", "pullRequest": null })
            );
        }

        #[test]
        fn a_failed_branch_carries_its_error() {
            let status = BranchPullRequestStatusView::from(PullRequestsStatus::Failed(
                GitHostingError::Timeout,
            ));

            let wire = serde_json::to_value(status).unwrap();

            assert_eq!(wire["kind"], "failed");
            assert_eq!(wire["error"]["kind"], "timeout");
        }

        #[test]
        fn statuses_without_pull_requests_are_sent_by_kind() {
            for (status, kind) in [
                (PullRequestsStatus::NoRemote, "noRemote"),
                (PullRequestsStatus::UnsupportedHost, "unsupportedHost"),
                (PullRequestsStatus::NotConnected, "notConnected"),
            ] {
                assert_eq!(status_wire(status), json!({ "kind": kind }));
            }
        }

        #[test]
        fn a_failed_repository_carries_its_error() {
            let status = status_wire(PullRequestsStatus::Failed(GitHostingError::Forbidden));

            assert_eq!(status["kind"], "failed");
            assert_eq!(status["error"]["kind"], "forbidden");
        }
    }

    #[test]
    fn slow_down_is_sent_with_its_interval() {
        let progress = SignInProgressView::from(SignInProgress::SlowDown {
            interval_seconds: INTERVAL_SECONDS,
        });

        assert_eq!(
            serde_json::to_value(progress).unwrap(),
            json!({ "status": "slowDown", "intervalSeconds": INTERVAL_SECONDS })
        );
    }
}
