//! GitHub adapter: device-flow sign-in and the REST API.

mod adapter;
mod client;
mod mapping;
mod pull_request_mapping;
mod wire;

use std::sync::Arc;

use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::ports::{BrowserSignIn, GitHost};

pub fn create_host(token: &HostSecret) -> Result<Box<dyn GitHost>, GitHostingError> {
    Ok(Box::new(adapter::GithubHost::new(
        client::GithubClient::new()?,
        HostSecret::new(token.expose()),
    )))
}

pub fn browser_sign_in() -> Result<Arc<dyn BrowserSignIn>, GitHostingError> {
    Ok(Arc::new(adapter::GithubBrowserSignIn::new(
        client::GithubClient::new()?,
    )))
}
