//! Lets `git` push and pull over HTTPS with a signed-in Git host. The token is
//! passed per call as an `http.<scope>.extraheader`, never written to
//! `.git/config`, and scoped to the Git host so no other remote receives it.

use std::sync::Arc;

use super::base64;
use crate::git::domain::ports::{RemoteAuth, RemoteCredentials};
use crate::git::domain::remote_url::RemoteUrl;
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::hosted_repository::HostedRepository;
use crate::git_hosting::domain::ports::HostTokens;

const HTTPS_SCHEME: &str = "https://";
/// GitHub accepts its access token as the Basic password with any username.
const GITHUB_BASIC_USERNAME: &str = "x-access-token";

pub struct HostTokenCredentials {
    host_tokens: Arc<dyn HostTokens>,
}

impl HostTokenCredentials {
    pub fn new(host_tokens: Arc<dyn HostTokens>) -> Self {
        Self { host_tokens }
    }
}

impl RemoteCredentials for HostTokenCredentials {
    /// SSH remotes authenticate with the user's keys, so only HTTPS remotes get a header.
    fn for_remote(&self, remote_url: &RemoteUrl) -> Option<RemoteAuth> {
        if !remote_url.as_str().starts_with(HTTPS_SCHEME) {
            return None;
        }
        let host = HostedRepository::from_remote(remote_url).ok()?.host;
        let secret = self.host_tokens.read(host).ok().flatten()?;
        let credential = basic_credential(host, &secret)?;
        Some(RemoteAuth {
            scope: format!("{HTTPS_SCHEME}{}/", host.host_name()),
            authorization: format!("Basic {}", base64::encode(credential.as_bytes())),
        })
    }
}

/// The `username:password` pair, before base64, git sends for `host`.
fn basic_credential(host: HostKind, secret: &HostSecret) -> Option<String> {
    match host {
        HostKind::Github => Some(format!("{GITHUB_BASIC_USERNAME}:{}", secret.expose())),
        HostKind::Bitbucket => secret
            .bitbucket_credential()
            .ok()
            .map(|credential| format!("{}:{}", credential.username, credential.token)),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;
    use crate::git_hosting::domain::ports::HostTokensUnavailable;

    const GITHUB_TOKEN: &str = "ghtoken";
    const BITBUCKET_USERNAME: &str = "alice";
    const BITBUCKET_TOKEN: &str = "app-pw";
    const GITHUB_HTTPS_REMOTE: &str = "https://github.com/FredPerr/me.git";
    const BITBUCKET_HTTPS_REMOTE: &str = "https://bitbucket.org/fredperr/me.git";
    const GITHUB_SCOPE: &str = "https://github.com/";
    const BITBUCKET_SCOPE: &str = "https://bitbucket.org/";

    #[derive(Default)]
    struct InMemoryHostTokens {
        secrets: Mutex<HashMap<HostKind, String>>,
        unavailable: bool,
    }

    impl HostTokens for InMemoryHostTokens {
        fn read(&self, host: HostKind) -> Result<Option<HostSecret>, HostTokensUnavailable> {
            if self.unavailable {
                return Err(HostTokensUnavailable);
            }
            Ok(self
                .secrets
                .lock()
                .unwrap()
                .get(&host)
                .map(|secret| HostSecret::new(secret.as_str())))
        }

        fn save(&self, host: HostKind, secret: &HostSecret) -> Result<(), HostTokensUnavailable> {
            self.secrets
                .lock()
                .unwrap()
                .insert(host, secret.expose().to_string());
            Ok(())
        }

        fn delete(&self, host: HostKind) -> Result<(), HostTokensUnavailable> {
            self.secrets.lock().unwrap().remove(&host);
            Ok(())
        }
    }

    fn signed_in_to_both_hosts() -> HostTokenCredentials {
        let host_tokens = InMemoryHostTokens::default();
        host_tokens
            .save(HostKind::Github, &HostSecret::new(GITHUB_TOKEN))
            .unwrap();
        host_tokens
            .save(
                HostKind::Bitbucket,
                &HostSecret::bitbucket(BITBUCKET_USERNAME, BITBUCKET_TOKEN),
            )
            .unwrap();
        HostTokenCredentials::new(Arc::new(host_tokens))
    }

    fn credentials_with(host_tokens: InMemoryHostTokens) -> HostTokenCredentials {
        HostTokenCredentials::new(Arc::new(host_tokens))
    }

    fn scope_and_authorization(
        credentials: &HostTokenCredentials,
        remote: &str,
    ) -> Option<(String, String)> {
        credentials
            .for_remote(&RemoteUrl::new(remote))
            .map(|auth| (auth.scope, auth.authorization))
    }

    fn basic_header(credential: &str) -> String {
        format!("Basic {}", base64::encode(credential.as_bytes()))
    }

    #[test]
    fn github_uses_x_access_token_as_the_basic_username() {
        assert_eq!(
            basic_credential(HostKind::Github, &HostSecret::new(GITHUB_TOKEN)),
            Some(format!("x-access-token:{GITHUB_TOKEN}"))
        );
    }

    #[test]
    fn bitbucket_uses_the_stored_username_and_token_pair() {
        let stored = format!("{BITBUCKET_USERNAME}:{BITBUCKET_TOKEN}");
        assert_eq!(
            basic_credential(HostKind::Bitbucket, &HostSecret::new(stored.as_str())),
            Some(stored)
        );
    }

    #[test]
    fn bitbucket_has_no_credential_for_a_malformed_secret() {
        assert_eq!(
            basic_credential(HostKind::Bitbucket, &HostSecret::new(BITBUCKET_TOKEN)),
            None
        );
    }

    #[test]
    fn an_https_github_remote_gets_the_token_scoped_to_github() {
        assert_eq!(
            scope_and_authorization(&signed_in_to_both_hosts(), GITHUB_HTTPS_REMOTE),
            Some((
                GITHUB_SCOPE.to_string(),
                basic_header(&format!("x-access-token:{GITHUB_TOKEN}"))
            ))
        );
    }

    #[test]
    fn an_https_bitbucket_remote_gets_the_credential_scoped_to_bitbucket() {
        assert_eq!(
            scope_and_authorization(&signed_in_to_both_hosts(), BITBUCKET_HTTPS_REMOTE),
            Some((
                BITBUCKET_SCOPE.to_string(),
                basic_header(&format!("{BITBUCKET_USERNAME}:{BITBUCKET_TOKEN}"))
            ))
        );
    }

    #[test]
    fn ssh_remotes_get_no_authorization() {
        let credentials = signed_in_to_both_hosts();
        for remote in [
            "git@github.com:FredPerr/me.git",
            "git@bitbucket.org:fredperr/me.git",
            "ssh://git@github.com/FredPerr/me.git",
        ] {
            assert!(
                scope_and_authorization(&credentials, remote).is_none(),
                "{remote}"
            );
        }
    }

    #[test]
    fn unsupported_remotes_get_no_authorization() {
        let credentials = signed_in_to_both_hosts();
        for remote in [
            "https://gitlab.com/FredPerr/me.git",
            "https://notgithub.com/FredPerr/me.git",
            "https://github.com/FredPerr",
        ] {
            assert!(
                scope_and_authorization(&credentials, remote).is_none(),
                "{remote}"
            );
        }
    }

    #[test]
    fn a_host_not_signed_in_gets_no_authorization() {
        let credentials = credentials_with(InMemoryHostTokens::default());
        assert!(scope_and_authorization(&credentials, GITHUB_HTTPS_REMOTE).is_none());
    }

    #[test]
    fn an_unavailable_keychain_gives_no_authorization() {
        let credentials = credentials_with(InMemoryHostTokens {
            unavailable: true,
            ..InMemoryHostTokens::default()
        });
        assert!(scope_and_authorization(&credentials, GITHUB_HTTPS_REMOTE).is_none());
    }
}
