/// A Git host's stored secret: a GitHub access token, or Bitbucket's
/// `username:token` pair. No `Debug`: it holds a secret.
pub struct HostSecret(String);

/// The two parts of a stored Bitbucket secret.
pub struct BitbucketCredential<'secret> {
    pub username: &'secret str,
    pub token: &'secret str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MalformedSecret;

impl HostSecret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The stored form of a Bitbucket sign-in, kept so existing sign-ins work.
    pub fn bitbucket(username: &str, token: &str) -> Self {
        Self(format!("{username}:{token}"))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Splits on the first colon; both parts must be present.
    pub fn bitbucket_credential(&self) -> Result<BitbucketCredential<'_>, MalformedSecret> {
        match self.0.split_once(':') {
            Some((username, token)) if !username.is_empty() && !token.is_empty() => {
                Ok(BitbucketCredential { username, token })
            }
            _ => Err(MalformedSecret),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const USERNAME: &str = "alice";
    const TOKEN: &str = "app-pw";
    const TOKEN_WITH_COLONS: &str = "tok:en:with:colons";

    fn split(stored: &str) -> Result<(String, String), MalformedSecret> {
        HostSecret::new(stored)
            .bitbucket_credential()
            .map(|credential| (credential.username.into(), credential.token.into()))
    }

    #[test]
    fn stores_a_bitbucket_sign_in_as_username_and_token() {
        assert_eq!(
            HostSecret::bitbucket(USERNAME, TOKEN).expose(),
            format!("{USERNAME}:{TOKEN}")
        );
    }

    #[test]
    fn splits_a_bitbucket_secret_into_username_and_token() {
        assert_eq!(
            split(&format!("{USERNAME}:{TOKEN}")),
            Ok((USERNAME.into(), TOKEN.into()))
        );
    }

    #[test]
    fn splits_only_on_the_first_colon() {
        assert_eq!(
            split(&format!("{USERNAME}:{TOKEN_WITH_COLONS}")),
            Ok((USERNAME.into(), TOKEN_WITH_COLONS.into()))
        );
    }

    #[test]
    fn rejects_a_secret_without_a_colon() {
        assert_eq!(split("no-colon"), Err(MalformedSecret));
    }

    #[test]
    fn rejects_a_secret_with_empty_parts() {
        assert_eq!(split(&format!(":{TOKEN}")), Err(MalformedSecret));
        assert_eq!(split(&format!("{USERNAME}:")), Err(MalformedSecret));
    }
}
