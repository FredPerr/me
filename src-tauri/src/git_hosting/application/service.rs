use std::collections::HashSet;
use std::sync::Arc;

use futures_util::future::join_all;

use super::branch_pull_request_cache::BranchPullRequestCache;
use super::registry::GitHostRegistry;
use crate::git::domain::remote_url::RemoteUrl;
use crate::git_hosting::domain::error::{GitHostingError, InputField};
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::hosted_account::{HostedAccount, SignedInHost};
use crate::git_hosting::domain::hosted_repository::HostedRepository;
use crate::git_hosting::domain::new_pull_request::{CreatedPullRequest, NewPullRequest};
use crate::git_hosting::domain::ports::{
    Clock, GitHost, GitHostingEvents, HostTokens, HostTokensUnavailable, HostedAccounts,
    HostedAccountsUnavailable, ProjectRemotes, PushFailure, WorktreePushes,
};
use crate::git_hosting::domain::project_pull_requests::{
    BranchPullRequest, BranchPullRequestStatus, ProjectRepositoryRemote, PullRequestsStatus,
    RepositoryPullRequests,
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

impl From<PushFailure> for GitHostingError {
    fn from(failure: PushFailure) -> Self {
        match failure {
            PushFailure::Skipped => Self::PushSkipped,
            PushFailure::Failed(reason) => Self::PushFailed { reason },
        }
    }
}

pub struct GitHostingService {
    registry: GitHostRegistry,
    host_tokens: Arc<dyn HostTokens>,
    accounts: Arc<dyn HostedAccounts>,
    events: Arc<dyn GitHostingEvents>,
    project_remotes: Arc<dyn ProjectRemotes>,
    worktree_pushes: Arc<dyn WorktreePushes>,
    branch_lookups: BranchPullRequestCache,
}

impl GitHostingService {
    pub fn new(
        registry: GitHostRegistry,
        host_tokens: Arc<dyn HostTokens>,
        accounts: Arc<dyn HostedAccounts>,
        events: Arc<dyn GitHostingEvents>,
        project_remotes: Arc<dyn ProjectRemotes>,
        worktree_pushes: Arc<dyn WorktreePushes>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            registry,
            host_tokens,
            accounts,
            events,
            project_remotes,
            worktree_pushes,
            branch_lookups: BranchPullRequestCache::new(clock),
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

    /// The pull request of each branch the Project's contexts use, once per
    /// repository and branch, in the Project's order. Lookups run at the same
    /// time and answers are reused for
    /// [`BRANCH_LOOKUP_LIFETIME`](super::branch_pull_request_cache::BRANCH_LOOKUP_LIFETIME).
    pub async fn context_branch_pull_requests(
        &self,
        project_tag: &str,
    ) -> Result<Vec<BranchPullRequest>, GitHostingError> {
        let repositories = self.project_remotes.repositories(project_tag).await?;
        let loads = repositories
            .into_iter()
            .map(|repository| self.repository_branch_pull_requests(repository));
        Ok(join_all(loads).await.into_iter().flatten().collect())
    }

    /// Opens a pull request from the branch a context of the Project uses in
    /// one of its repositories: the Git host comes from the repository's
    /// remote, and the branch is pushed first so the Git host has it. The
    /// branch's cached lookup is forgotten, so the board shows the new pull
    /// request right away.
    pub async fn create_pull_request(
        &self,
        project_tag: &str,
        repository_id: &str,
        pull_request: NewPullRequest,
    ) -> Result<CreatedPullRequest, GitHostingError> {
        let pull_request = pull_request.validated()?;
        let checkout = self
            .project_remotes
            .branch_checkout(project_tag, repository_id, &pull_request.head)
            .await?;
        let repository = match checkout.remote.as_ref().map(HostedRepository::from_remote) {
            None => return Err(GitHostingError::NoRemote),
            Some(Err(_)) => return Err(GitHostingError::UnsupportedHost),
            Some(Ok(repository)) => repository,
        };
        let git_host = self.connected_host(repository.host)?;
        self.worktree_pushes
            .push(&checkout.worktree_path)
            .await
            .map_err(|failure| {
                eprintln!(
                    "[git_hosting] branch {} of {} not pushed: {failure:?}",
                    pull_request.head,
                    describe(&repository)
                );
                GitHostingError::from(failure)
            })?;
        let created = git_host
            .create_pull_request(&repository, &pull_request)
            .await
            .inspect_err(|error| {
                eprintln!(
                    "[git_hosting] pull request from {} of {} not created: {error:?}",
                    pull_request.head,
                    describe(&repository)
                );
            })?;
        self.forget_branch(&repository, &pull_request.head);
        Ok(created)
    }

    /// `git` pushed `branch` to `remote`: its pull request may have changed,
    /// so the next lookup asks the Git host again. A remote on no supported
    /// Git host has nothing cached.
    pub fn branch_pushed(&self, remote: &RemoteUrl, branch: &str) {
        if let Ok(repository) = HostedRepository::from_remote(remote) {
            self.forget_branch(&repository, branch);
        }
    }

    /// Asks the Git host again on the next lookup, e.g. once a pull request
    /// was created from the branch or the branch was pushed.
    pub fn forget_branch(&self, repository: &HostedRepository, branch: &str) {
        self.branch_lookups.forget(repository, branch);
    }

    async fn repository_pull_requests(
        &self,
        repository: ProjectRepositoryRemote,
    ) -> RepositoryPullRequests {
        let (host, status) = match hosted_repository(repository.remote.as_ref()) {
            Err(status) => (None, status),
            Ok(hosted) => (Some(hosted.host), self.open_pull_requests(&hosted).await),
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
            Err(error) => failed_status(error, || {
                format!("pull requests of {} not listed", describe(repository))
            }),
        }
    }

    async fn repository_branch_pull_requests(
        &self,
        repository: ProjectRepositoryRemote,
    ) -> Vec<BranchPullRequest> {
        let branches = distinct(repository.context_branches);
        let (host, statuses) = match hosted_repository(repository.remote.as_ref()) {
            Err(status) => (None, vec![status; branches.len()]),
            Ok(hosted) => (
                Some(hosted.host),
                self.branch_statuses(&hosted, &branches).await,
            ),
        };
        branches
            .into_iter()
            .zip(statuses)
            .map(|(branch, status)| BranchPullRequest {
                repository_id: repository.repository_id.clone(),
                branch,
                host,
                status,
            })
            .collect()
    }

    /// One status per branch, in the order of `branches`. The Git host is
    /// signed in to once for the repository.
    async fn branch_statuses(
        &self,
        repository: &HostedRepository,
        branches: &[String],
    ) -> Vec<BranchPullRequestStatus> {
        if branches.is_empty() {
            return Vec::new();
        }
        let git_host = match self.connected_host(repository.host) {
            Ok(git_host) => git_host,
            Err(error) => {
                let status = failed_status(error, || {
                    format!("branch pull requests of {} not found", describe(repository))
                });
                return vec![status; branches.len()];
            }
        };
        let lookups = branches
            .iter()
            .map(|branch| self.branch_status(git_host.as_ref(), repository, branch));
        join_all(lookups).await
    }

    async fn branch_status(
        &self,
        git_host: &dyn GitHost,
        repository: &HostedRepository,
        branch: &str,
    ) -> BranchPullRequestStatus {
        if let Some(pull_request) = self.branch_lookups.find(repository, branch) {
            return PullRequestsStatus::Ok(pull_request);
        }
        match git_host.find_branch_pull_request(repository, branch).await {
            Ok(pull_request) => {
                self.branch_lookups
                    .remember(repository, branch, pull_request.clone());
                PullRequestsStatus::Ok(pull_request)
            }
            Err(error) => failed_status(error, || {
                format!(
                    "pull request of branch {branch} of {} not found",
                    describe(repository)
                )
            }),
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

/// The repository on a supported Git host, or the status saying why there is none.
fn hosted_repository<Found>(
    remote: Option<&RemoteUrl>,
) -> Result<HostedRepository, PullRequestsStatus<Found>> {
    match remote.map(HostedRepository::from_remote) {
        None => Err(PullRequestsStatus::NoRemote),
        Some(Err(_)) => Err(PullRequestsStatus::UnsupportedHost),
        Some(Ok(hosted)) => Ok(hosted),
    }
}

/// `NotConnected` is a status of its own; any other error is logged.
fn failed_status<Found>(
    error: GitHostingError,
    failure: impl FnOnce() -> String,
) -> PullRequestsStatus<Found> {
    match error {
        GitHostingError::NotConnected => PullRequestsStatus::NotConnected,
        error => {
            eprintln!("[git_hosting] {}: {error:?}", failure());
            PullRequestsStatus::Failed(error)
        }
    }
}

fn describe(repository: &HostedRepository) -> String {
    format!(
        "{}/{} on {}",
        repository.owner,
        repository.name,
        repository.host.as_str()
    )
}

/// Each branch once, in the order first seen.
fn distinct(branches: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    branches
        .into_iter()
        .filter(|branch| seen.insert(branch.clone()))
        .collect()
}

fn required(value: &str, field: InputField) -> Result<&str, GitHostingError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(GitHostingError::invalid_input(field))
    } else {
        Ok(trimmed)
    }
}
