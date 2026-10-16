use crate::git::domain::ports::{RemoteAuth, RemoteCredentials};
use crate::git::domain::remote_url::RemoteUrl;
use crate::git_provider::git_auth;

/// Credentials from the GitHub and Bitbucket tokens stored in the keychain.
pub struct ProviderTokenCredentials;

impl RemoteCredentials for ProviderTokenCredentials {
    fn for_remote(&self, remote_url: &RemoteUrl) -> Option<RemoteAuth> {
        git_auth::authorization_for_remote(remote_url.as_str()).map(|(scope, authorization)| {
            RemoteAuth {
                scope,
                authorization,
            }
        })
    }
}
