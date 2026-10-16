use std::collections::HashMap;
use std::sync::Arc;

use crate::git_hosting::domain::error::{GitHostingError, InputField};
use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::ports::{BrowserSignIn, GitHost};

pub type GitHostFactory =
    Arc<dyn Fn(&HostSecret) -> Result<Box<dyn GitHost>, GitHostingError> + Send + Sync>;

/// Maps each Git host to the factory building its adapter from a secret, and
/// to its browser sign-in when it has one.
#[derive(Default)]
pub struct GitHostRegistry {
    hosts: HashMap<HostKind, GitHostFactory>,
    browser_sign_ins: HashMap<HostKind, Arc<dyn BrowserSignIn>>,
}

impl GitHostRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_host(mut self, kind: HostKind, factory: GitHostFactory) -> Self {
        self.hosts.insert(kind, factory);
        self
    }

    pub fn register_browser_sign_in(
        mut self,
        kind: HostKind,
        browser_sign_in: Arc<dyn BrowserSignIn>,
    ) -> Self {
        self.browser_sign_ins.insert(kind, browser_sign_in);
        self
    }

    pub fn create(
        &self,
        kind: HostKind,
        secret: &HostSecret,
    ) -> Result<Box<dyn GitHost>, GitHostingError> {
        let factory = self
            .hosts
            .get(&kind)
            .ok_or_else(|| GitHostingError::invalid_input(InputField::Host))?;
        factory(secret)
    }

    pub fn browser_sign_in(&self, kind: HostKind) -> Result<&dyn BrowserSignIn, GitHostingError> {
        self.browser_sign_ins
            .get(&kind)
            .map(|browser_sign_in| browser_sign_in.as_ref())
            .ok_or_else(|| GitHostingError::invalid_input(InputField::Host))
    }
}
