//! Bitbucket Cloud adapter: the REST API, signed in with a username and an API token.

mod adapter;
mod client;
mod mapping;
mod new_pull_request_mapping;
mod pull_request_mapping;
mod wire;

use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::ports::GitHost;

/// A malformed stored secret cannot sign in, so it counts as not connected.
pub fn create_host(secret: &HostSecret) -> Result<Box<dyn GitHost>, GitHostingError> {
    let credential = secret
        .bitbucket_credential()
        .map_err(|_| GitHostingError::NotConnected)?;
    Ok(Box::new(adapter::BitbucketHost::new(
        client::BitbucketClient::new(credential.token)?,
        credential.username,
    )))
}
