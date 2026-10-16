use super::host_kind::HostKind;
use super::host_secret::HostSecret;

/// How a Git host signs a user in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInMethod {
    /// The user enters a code in the browser (GitHub device flow).
    Browser,
    /// The user enters a username and a token (Bitbucket API token).
    Token,
}

impl HostKind {
    pub fn sign_in_method(&self) -> SignInMethod {
        match self {
            Self::Github => SignInMethod::Browser,
            Self::Bitbucket => SignInMethod::Token,
        }
    }
}

/// What the user must do in the browser, and how to wait for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSignInStep {
    pub user_code: String,
    pub verification_url: String,
    pub device_code: String,
    pub interval_seconds: u32,
    pub expires_in_seconds: u32,
}

/// One check of a browser sign-in. No `Debug`: `Authorized` holds a secret.
pub enum SignInPoll {
    Authorized(HostSecret),
    Pending,
    SlowDown { interval_seconds: u32 },
    Expired,
    Denied,
}

/// A browser sign-in check as the application reports it, without the secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInProgress {
    Authorized,
    Pending,
    SlowDown { interval_seconds: u32 },
    Expired,
    Denied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_signs_in_through_the_browser() {
        assert_eq!(HostKind::Github.sign_in_method(), SignInMethod::Browser);
    }

    #[test]
    fn bitbucket_signs_in_with_a_token() {
        assert_eq!(HostKind::Bitbucket.sign_in_method(), SignInMethod::Token);
    }
}
