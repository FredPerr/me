//! Typed Tauri commands of the Git Hosting context and their wire types.
//! Secrets never leave Rust: sign-in commands only return what the UI shows.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::application::service::GitHostingService;
use super::domain::error::{GitHostingError, InputField};
use super::domain::host_kind::HostKind;
use super::domain::hosted_account::{HostedAccount, SignedInHost};
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

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
    fn hosts_are_sent_by_identifier() {
        for host in HostKind::ALL {
            assert_eq!(
                serde_json::to_value(HostKindView::from(host)).unwrap(),
                json!(host.as_str())
            );
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
