//! Tells `git_hosting` about the branches `git` pushed, so their cached pull
//! request lookups are read again.

use std::sync::Arc;

use crate::git::domain::branch_name::BranchName;
use crate::git::domain::ports::BranchPushListener;
use crate::git::domain::remote_url::RemoteUrl;
use crate::git_hosting::application::service::GitHostingService;

pub struct GitHostingPushListener {
    git_hosting: Arc<GitHostingService>,
}

impl GitHostingPushListener {
    pub fn new(git_hosting: Arc<GitHostingService>) -> Self {
        Self { git_hosting }
    }
}

impl BranchPushListener for GitHostingPushListener {
    fn branch_pushed(&self, remote: &RemoteUrl, branch: &BranchName) {
        self.git_hosting.branch_pushed(remote, branch.as_str());
    }
}
