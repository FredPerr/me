//! Fakes for the application-layer tests. Each test owns its own instances
//! (no statics), so tests run in parallel.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::registry::GitHostFactory;
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::hosted_account::HostedAccount;
use crate::git_hosting::domain::ports::{
    BrowserSignIn, GitHost, GitHostingEvents, HostTokens, HostTokensUnavailable,
};
use crate::git_hosting::domain::sign_in::{BrowserSignInStep, SignInPoll};

pub const GITHUB_LOGIN: &str = "octocat";
pub const BITBUCKET_LOGIN: &str = "alice";
pub const ACCOUNT_NAME: &str = "Alice Example";
pub const AVATAR_URL: &str = "https://avatars.example/alice.png";
pub const USER_CODE: &str = "WDJB-MJHT";
pub const VERIFICATION_URL: &str = "https://github.com/login/device";
pub const DEVICE_CODE: &str = "device-code-1";
pub const INTERVAL_SECONDS: u32 = 5;
pub const EXPIRES_IN_SECONDS: u32 = 900;

pub fn account(host: HostKind, login: &str) -> HostedAccount {
    HostedAccount {
        host,
        login: login.to_string(),
        name: Some(ACCOUNT_NAME.to_string()),
        avatar_url: Some(AVATAR_URL.to_string()),
    }
}

pub fn browser_step() -> BrowserSignInStep {
    BrowserSignInStep {
        user_code: USER_CODE.to_string(),
        verification_url: VERIFICATION_URL.to_string(),
        device_code: DEVICE_CODE.to_string(),
        interval_seconds: INTERVAL_SECONDS,
        expires_in_seconds: EXPIRES_IN_SECONDS,
    }
}

/// Secrets in memory; `unavailable` makes every call fail like a locked keychain.
#[derive(Default)]
pub struct FakeHostTokens {
    secrets: Mutex<HashMap<HostKind, String>>,
    unavailable: bool,
}

impl FakeHostTokens {
    pub fn unavailable() -> Self {
        Self {
            unavailable: true,
            ..Self::default()
        }
    }

    pub fn with_secret(host: HostKind, secret: &str) -> Self {
        let host_tokens = Self::default();
        host_tokens
            .secrets
            .lock()
            .unwrap()
            .insert(host, secret.to_string());
        host_tokens
    }

    pub fn secret(&self, host: HostKind) -> Option<String> {
        self.secrets.lock().unwrap().get(&host).cloned()
    }

    fn available(&self) -> Result<(), HostTokensUnavailable> {
        if self.unavailable {
            Err(HostTokensUnavailable)
        } else {
            Ok(())
        }
    }
}

impl HostTokens for FakeHostTokens {
    fn read(&self, host: HostKind) -> Result<Option<HostSecret>, HostTokensUnavailable> {
        self.available()?;
        Ok(self.secret(host).map(HostSecret::new))
    }

    fn save(&self, host: HostKind, secret: &HostSecret) -> Result<(), HostTokensUnavailable> {
        self.available()?;
        self.secrets
            .lock()
            .unwrap()
            .insert(host, secret.expose().to_string());
        Ok(())
    }

    fn delete(&self, host: HostKind) -> Result<(), HostTokensUnavailable> {
        self.available()?;
        self.secrets.lock().unwrap().remove(&host);
        Ok(())
    }
}

/// Scripted `current_account` answer of one Git host, plus a log of the
/// secrets its adapters were built with and of the account fetches.
pub struct HostScript {
    pub account_result: Result<HostedAccount, GitHostingError>,
    pub created_with_secrets: Vec<String>,
    pub account_fetches: usize,
}

impl HostScript {
    pub fn shared(account_result: Result<HostedAccount, GitHostingError>) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            account_result,
            created_with_secrets: Vec::new(),
            account_fetches: 0,
        }))
    }

    pub fn factory(script: &Arc<Mutex<Self>>) -> GitHostFactory {
        let script = Arc::clone(script);
        Arc::new(move |secret: &HostSecret| {
            script
                .lock()
                .unwrap()
                .created_with_secrets
                .push(secret.expose().to_string());
            Ok(Box::new(ScriptedGitHost {
                script: Arc::clone(&script),
            }) as Box<dyn GitHost>)
        })
    }
}

struct ScriptedGitHost {
    script: Arc<Mutex<HostScript>>,
}

#[async_trait]
impl GitHost for ScriptedGitHost {
    async fn current_account(&self) -> Result<HostedAccount, GitHostingError> {
        let mut script = self.script.lock().unwrap();
        script.account_fetches += 1;
        script.account_result.clone()
    }
}

/// A browser sign-in check to answer; `Authorized` carries the issued token.
#[derive(Clone, Copy)]
pub enum ScriptedPoll {
    Authorized(&'static str),
    Pending,
    SlowDown(u32),
    Expired,
    Denied,
    Fails(GitHostingError),
}

pub struct BrowserScript {
    pub start_result: Result<BrowserSignInStep, GitHostingError>,
    pub poll: ScriptedPoll,
    pub polled_device_codes: Vec<String>,
}

impl BrowserScript {
    pub fn shared(poll: ScriptedPoll) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            start_result: Ok(browser_step()),
            poll,
            polled_device_codes: Vec::new(),
        }))
    }
}

pub struct ScriptedBrowserSignIn {
    pub script: Arc<Mutex<BrowserScript>>,
}

#[async_trait]
impl BrowserSignIn for ScriptedBrowserSignIn {
    async fn start(&self) -> Result<BrowserSignInStep, GitHostingError> {
        self.script.lock().unwrap().start_result.clone()
    }

    async fn poll(&self, device_code: &str) -> Result<SignInPoll, GitHostingError> {
        let mut script = self.script.lock().unwrap();
        script.polled_device_codes.push(device_code.to_string());
        match script.poll {
            ScriptedPoll::Authorized(token) => Ok(SignInPoll::Authorized(HostSecret::new(token))),
            ScriptedPoll::Pending => Ok(SignInPoll::Pending),
            ScriptedPoll::SlowDown(interval_seconds) => {
                Ok(SignInPoll::SlowDown { interval_seconds })
            }
            ScriptedPoll::Expired => Ok(SignInPoll::Expired),
            ScriptedPoll::Denied => Ok(SignInPoll::Denied),
            ScriptedPoll::Fails(error) => Err(error),
        }
    }
}

#[derive(Default)]
pub struct RecordingEvents {
    changed: Mutex<Vec<HostKind>>,
}

impl RecordingEvents {
    pub fn changed_hosts(&self) -> Vec<HostKind> {
        self.changed.lock().unwrap().clone()
    }
}

impl GitHostingEvents for RecordingEvents {
    fn changed(&self, host: HostKind) {
        self.changed.lock().unwrap().push(host);
    }
}
