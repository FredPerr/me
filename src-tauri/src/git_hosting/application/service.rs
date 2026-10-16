use std::sync::Arc;

use futures_util::future::join_all;

use super::registry::GitHostRegistry;
use crate::git_hosting::domain::error::{GitHostingError, InputField};
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::hosted_account::{HostedAccount, SignedInHost};
use crate::git_hosting::domain::hosted_repository::HostedRepository;
use crate::git_hosting::domain::ports::{
    GitHost, GitHostingEvents, HostTokens, HostTokensUnavailable, HostedAccounts,
    HostedAccountsUnavailable, ProjectRemotes,
};
use crate::git_hosting::domain::project_pull_requests::{
    ProjectRepositoryRemote, PullRequestsStatus, RepositoryPullRequests,
};
use crate::git_hosting::domain::sign_in::{
    BrowserSignInStep, SignInMethod, SignInPoll, SignInProgress,
};

impl From<HostTokensUnavailable> for GitHostingError {
    fn from(_: HostTokensUnavailable) -> Self {
        Self::Storage
    }
}

impl From<HostedAccountsUnavailable> for GitHostingError {
    fn from(_: HostedAccountsUnavailable) -> Self {
        Self::Storage
    }
}

pub struct GitHostingService {
    registry: GitHostRegistry,
    host_tokens: Arc<dyn HostTokens>,
    accounts: Arc<dyn HostedAccounts>,
    events: Arc<dyn GitHostingEvents>,
    project_remotes: Arc<dyn ProjectRemotes>,
}

impl GitHostingService {
    pub fn new(
        registry: GitHostRegistry,
        host_tokens: Arc<dyn HostTokens>,
        accounts: Arc<dyn HostedAccounts>,
        events: Arc<dyn GitHostingEvents>,
        project_remotes: Arc<dyn ProjectRemotes>,
    ) -> Self {
        Self {
            registry,
            host_tokens,
            accounts,
            events,
            project_remotes,
        }
    }

    /// Every Git host with a stored secret. An account missing from storage
    /// (signed in before accounts were stored) is fetched, saved and returned;
    /// when that fetch fails the host is listed without an account, so one
    /// unreachable Git host never hides the others.
    pub async fn list_accounts(&self) -> Result<Vec<SignedInHost>, GitHostingError> {
        let stored_accounts = self.accounts.list()?;
        let mut signed_in_hosts = Vec::new();
        for host in HostKind::ALL {
            if self.host_tokens.read(host)?.is_none() {
                continue;
            }
            let stored = stored_accounts
                .iter()
                .find(|account| account.host == host)
                .cloned();
            let account = match stored {
                Some(account) => Some(account),
                None => self.fetch_and_save_account(host).await,
            };
            signed_in_hosts.push(SignedInHost { host, account });
        }
        Ok(signed_in_hosts)
    }

    pub async fn start_browser_sign_in(
        &self,
        host: HostKind,
    ) -> Result<BrowserSignInStep, GitHostingError> {
        self.require_sign_in_method(host, SignInMethod::Browser)?;
        self.registry.browser_sign_in(host)?.start().await
    }

    /// On `Authorized` the secret is saved and the account fetched and saved.
    /// A failed account fetch is logged and left to `list_accounts`.
    pub async fn poll_browser_sign_in(
        &self,
        host: HostKind,
        device_code: &str,
    ) -> Result<SignInProgress, GitHostingError> {
        self.require_sign_in_method(host, SignInMethod::Browser)?;
        let device_code = required(device_code, InputField::DeviceCode)?;
        let poll = self
            .registry
            .browser_sign_in(host)?
            .poll(device_code)
            .await?;
        Ok(match poll {
            SignInPoll::Authorized(secret) => {
                self.host_tokens.save(host, &secret)?;
                self.accounts.remove(host)?;
                self.fetch_and_save_account(host).await;
                self.events.changed(host);
                SignInProgress::Authorized
            }
            SignInPoll::Pending => SignInProgress::Pending,
            SignInPoll::SlowDown { interval_seconds } => {
                SignInProgress::SlowDown { interval_seconds }
            }
            SignInPoll::Expired => SignInProgress::Expired,
            SignInPoll::Denied => SignInProgress::Denied,
        })
    }

    /// The candidate secret is checked with the current account before
    /// anything is saved, so a wrong credential never stays behind.
    pub async fn sign_in_with_token(
        &self,
        host: HostKind,
        username: &str,
        token: &str,
    ) -> Result<HostedAccount, GitHostingError> {
        self.require_sign_in_method(host, SignInMethod::Token)?;
        let username = required(username, InputField::Username)?;
        let token = required(token, InputField::Token)?;
        let candidate = HostSecret::bitbucket(username, token);
        let account = self
            .registry
            .create(host, &candidate)?
            .current_account()
            .await?;
        self.host_tokens.save(host, &candidate)?;
        self.accounts.save(&account)?;
        self.events.changed(host);
        Ok(account)
    }

    /// Deletes the secret and the account. Signing out twice is not an error.
    pub async fn sign_out(&self, host: HostKind) -> Result<(), GitHostingError> {
        self.host_tokens.delete(host)?;
        self.accounts.remove(host)?;
        self.events.changed(host);
        Ok(())
    }

    /// The Git host's API signed in with its stored secret.
    pub fn connected_host(&self, host: HostKind) -> Result<Box<dyn GitHost>, GitHostingError> {
        let secret = self
            .host_tokens
            .read(host)?
            .ok_or(GitHostingError::NotConnected)?;
        self.registry.create(host, &secret)
    }

    /// The open pull requests of every repository of the Project, in the
    /// Project's order. Repositories load at the same time and each gets its
    /// own status, so one failing repository never hides the others.
    pub async fn project_open_pull_requests(
        &self,
        project_tag: &str,
    ) -> Result<Vec<RepositoryPullRequests>, GitHostingError> {
        let repositories = self.project_remotes.repositories(project_tag).await?;
        let loads = repositories
            .into_iter()
            .map(|repository| self.repository_pull_requests(repository));
        Ok(join_all(loads).await)
    }

    async fn repository_pull_requests(
        &self,
        repository: ProjectRepositoryRemote,
    ) -> RepositoryPullRequests {
        let (host, status) = match repository
            .remote
            .as_ref()
            .map(HostedRepository::from_remote)
        {
            None => (None, PullRequestsStatus::NoRemote),
            Some(Err(_)) => (None, PullRequestsStatus::UnsupportedHost),
            Some(Ok(hosted)) => (Some(hosted.host), self.open_pull_requests(&hosted).await),
        };
        RepositoryPullRequests {
            repository_id: repository.repository_id,
            repository_name: repository.repository_name,
            host,
            status,
        }
    }

    async fn open_pull_requests(&self, repository: &HostedRepository) -> PullRequestsStatus {
        let listed = match self.connected_host(repository.host) {
            Ok(git_host) => git_host.list_open_pull_requests(repository).await,
            Err(error) => Err(error),
        };
        match listed {
            Ok(pull_requests) => PullRequestsStatus::Ok(pull_requests),
            Err(GitHostingError::NotConnected) => PullRequestsStatus::NotConnected,
            Err(error) => {
                eprintln!(
                    "[git_hosting] pull requests of {}/{} on {} not listed: {error:?}",
                    repository.owner,
                    repository.name,
                    repository.host.as_str()
                );
                PullRequestsStatus::Failed(error)
            }
        }
    }

    fn require_sign_in_method(
        &self,
        host: HostKind,
        method: SignInMethod,
    ) -> Result<(), GitHostingError> {
        if host.sign_in_method() == method {
            Ok(())
        } else {
            Err(GitHostingError::invalid_input(InputField::Host))
        }
    }

    async fn fetch_and_save_account(&self, host: HostKind) -> Option<HostedAccount> {
        let fetched = match self.connected_host(host) {
            Ok(git_host) => git_host.current_account().await,
            Err(error) => Err(error),
        };
        let saved = fetched.and_then(|account| {
            self.accounts.save(&account)?;
            Ok(account)
        });
        match saved {
            Ok(account) => Some(account),
            Err(error) => {
                eprintln!(
                    "[git_hosting] account of {} not fetched: {error:?}",
                    host.as_str()
                );
                None
            }
        }
    }
}

fn required(value: &str, field: InputField) -> Result<&str, GitHostingError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(GitHostingError::invalid_input(field))
    } else {
        Ok(trimmed)
    }
}
