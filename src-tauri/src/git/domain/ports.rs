use std::path::Path;

use super::branch_name::BranchName;
use super::error::GitError;
use super::paths::{RepositoryPath, WorktreePath};
use super::remote_url::RemoteUrl;

/// A local branch and what the repository knows about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBranch {
    pub name: BranchName,
    /// HEAD of the main checkout.
    pub is_current: bool,
    /// Checked out in the main checkout or any worktree, so it cannot be deleted
    /// without removing that checkout first.
    pub is_checked_out: bool,
    pub has_upstream: bool,
    /// Had an upstream whose remote branch no longer exists.
    pub upstream_gone: bool,
    /// Merged into the repository's HEAD.
    pub is_merged: bool,
}

/// A checkout of the repository: the main one or a linked worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkout {
    pub path: RepositoryPath,
    pub branch: Option<BranchName>,
    pub is_main: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffStats {
    pub files_added: usize,
    pub files_modified: usize,
    pub files_deleted: usize,
    pub files_renamed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

/// Opens repositories. Implemented once by the git adapter and by fakes in tests.
pub trait GitRepositories: Send + Sync {
    fn open(&self, path: &RepositoryPath) -> Result<Box<dyn GitRepository>, GitError>;

    /// Whether `path` is the root folder of a repository's main checkout.
    fn is_repository_root(&self, path: &Path) -> bool;
}

/// One repository, opened at its main checkout or at one of its worktrees.
/// Every method that changes the repository is serialized per repository.
pub trait GitRepository {
    fn current_branch(&self) -> Option<BranchName>;
    fn has_branch(&self, name: &BranchName) -> bool;
    /// Local branch names, sorted.
    fn branch_names(&self) -> Result<Vec<BranchName>, GitError>;
    /// Local branches with their state, sorted by name.
    fn branches(&self) -> Result<Vec<LocalBranch>, GitError>;
    /// The main checkout first, then every linked worktree.
    fn worktrees(&self) -> Result<Vec<Checkout>, GitError>;
    /// Where `branch` is checked out, if anywhere.
    #[allow(dead_code)] // first caller arrives with the workspace module
    fn worktree_for(&self, branch: &BranchName) -> Result<Option<WorktreePath>, GitError>;
    /// The `origin` remote, if configured.
    fn remote_url(&self) -> Option<RemoteUrl>;
    /// Changes of HEAD since its merge base with `base`, or the uncommitted
    /// changes when there is no base.
    fn diff_stats_against(&self, base: Option<&BranchName>) -> Result<DiffStats, GitError>;
    /// Unified diff of HEAD since its merge base with `base`.
    fn diff_against(&self, base: &BranchName) -> Result<String, GitError>;

    fn create_branch(&self, name: &BranchName, from: &BranchName) -> Result<(), GitError>;
    fn delete_branch(&self, name: &BranchName, force: bool) -> Result<(), GitError>;
    /// Checks `branch` out at `path`, or returns the existing worktree when the
    /// branch is already checked out somewhere.
    fn add_worktree(
        &self,
        path: &WorktreePath,
        branch: &BranchName,
    ) -> Result<WorktreePath, GitError>;
    /// Removes the worktree at `path` and its folder.
    fn remove_worktree(&self, path: &WorktreePath) -> Result<(), GitError>;
    fn prune_worktrees(&self) -> Result<(), GitError>;
    /// Fetches `name` from `origin` into a local branch of the same name. Does
    /// nothing when the local branch already exists.
    fn fetch_branch(&self, name: &BranchName) -> Result<(), GitError>;
    /// Pushes the current branch to its upstream; returns git's report.
    fn push(&self) -> Result<String, GitError>;
    /// Fast-forwards the current branch to its upstream; returns git's report.
    fn pull(&self) -> Result<String, GitError>;
}

/// An HTTP authorization header for one remote host. No `Debug`: it holds a secret.
pub struct RemoteAuth {
    /// The URL prefix the header is scoped to, e.g. `https://github.com/`.
    pub scope: String,
    /// The full header value, e.g. `Basic dXNlcjp0b2tlbg==`.
    pub authorization: String,
}

/// Credentials for network operations on a remote.
pub trait RemoteCredentials: Send + Sync {
    fn for_remote(&self, remote_url: &RemoteUrl) -> Option<RemoteAuth>;
}
