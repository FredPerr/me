//! Git Hosting bounded context: Git hosts such as GitHub and Bitbucket, the
//! repositories they host and the accounts signed in to them.
//!
//! This module is the composition root: the only place that names the GitHub
//! and Bitbucket adapters, the keychain, the redb account store, the Tauri
//! event adapter and the credentials handed to the `git` module.

pub mod application;
pub mod commands;
pub mod domain;
pub mod infra;

use std::sync::Arc;

use application::registry::GitHostRegistry;
use application::service::GitHostingService;
use domain::host_kind::HostKind;
use infra::host_token_credentials::HostTokenCredentials;
use infra::keychain_host_tokens::KeychainHostTokens;
use infra::redb_hosted_accounts::RedbHostedAccounts;
use infra::tauri_git_hosting_events::TauriGitHostingEvents;

use crate::git::domain::ports::RemoteCredentials;

/// The Git Hosting service, held in Tauri state.
pub struct GitHosting {
    pub service: Arc<GitHostingService>,
}

pub fn build(
    app: tauri::AppHandle,
    database: Arc<redb::Database>,
) -> Result<GitHosting, Box<dyn std::error::Error>> {
    let accounts = RedbHostedAccounts::new(database)
        .map_err(|_| "the Git hosting account table could not be opened")?;
    let browser_sign_in = infra::github::browser_sign_in()
        .map_err(|_| "the GitHub sign-in client could not be built")?;
    let registry = GitHostRegistry::new()
        .register_host(HostKind::Github, Arc::new(infra::github::create_host))
        .register_host(HostKind::Bitbucket, Arc::new(infra::bitbucket::create_host))
        .register_browser_sign_in(HostKind::Github, browser_sign_in);
    let service = GitHostingService::new(
        registry,
        Arc::new(KeychainHostTokens),
        Arc::new(accounts),
        Arc::new(TauriGitHostingEvents::new(app)),
    );
    Ok(GitHosting {
        service: Arc::new(service),
    })
}

/// Credentials `git` uses to push and pull over HTTPS with a signed-in Git host.
pub fn remote_credentials() -> Arc<dyn RemoteCredentials> {
    Arc::new(HostTokenCredentials::new(Arc::new(KeychainHostTokens)))
}
