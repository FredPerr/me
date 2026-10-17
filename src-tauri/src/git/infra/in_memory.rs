//! An in-memory `GitRepositories` for tests of code that uses git. Each
//! repository holds its branches (as commit histories), its checkouts and an
//! optional `origin` remote; any operation can be told to fail. Each test owns
//! its own instance, so tests run in parallel.
//!
//! Files are not modelled: diffs are empty and worktree folders exist only as
//! entries, which `delete_worktree_folder` can mark as gone.
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::git::domain::branch_name::BranchName;
use crate::git::domain::error::GitError;
use crate::git::domain::paths::{RepositoryPath, WorktreePath};
use crate::git::domain::ports::{Checkout, DiffStats, GitRepositories, GitRepository, LocalBranch};
use crate::git::domain::remote_url::RemoteUrl;

/// A `GitRepositories` or `GitRepository` operation that can be told to fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Open,
    BranchNames,
    Branches,
    Worktrees,
    WorktreeFor,
    DiffStats,
    Diff,
    CreateBranch,
    DeleteBranch,
    HasCommitsBeyond,
    AddWorktree,
    RemoveWorktree,
    PruneWorktrees,
    FetchBranch,
    Push,
    Pull,
}

type CommitId = String;
type History = Vec<CommitId>;

#[derive(Clone)]
struct Branch {
    name: BranchName,
    /// Oldest first; never empty.
    history: History,
    /// The tip when the branch was created, fetched or added.
    base_commit: CommitId,
    /// The remote branch it tracks, once published.
    upstream: Option<BranchName>,
}

impl Branch {
    fn starting_at(name: BranchName, history: History) -> Self {
        let base_commit = history.last().expect("a branch has a commit").clone();
        Self {
            name,
            history,
            base_commit,
            upstream: None,
        }
    }

    fn tip(&self) -> &CommitId {
        self.history.last().expect("a branch has a commit")
    }

    fn contains(&self, commit: &CommitId) -> bool {
        self.history.contains(commit)
    }
}

struct Worktree {
    path: WorktreePath,
    branch: BranchName,
    folder_deleted: bool,
}

struct Remote {
    url: RemoteUrl,
    branches: BTreeMap<String, History>,
}

/// The branch checked out at a checkout and the remote branch it tracks.
struct Tracking {
    local: Branch,
    remote_history: History,
}

struct Repository {
    main_path: RepositoryPath,
    main_branch: Option<BranchName>,
    branches: BTreeMap<String, Branch>,
    worktrees: Vec<Worktree>,
    remote: Option<Remote>,
}

impl Repository {
    fn branch(&self, name: &BranchName) -> Result<&Branch, GitError> {
        self.branches
            .get(name.as_str())
            .ok_or_else(|| GitError::BranchNotFound {
                name: name.to_string(),
            })
    }

    fn branch_mut(&mut self, name: &BranchName) -> Result<&mut Branch, GitError> {
        self.branches
            .get_mut(name.as_str())
            .ok_or_else(|| GitError::BranchNotFound {
                name: name.to_string(),
            })
    }

    fn remote(&self) -> Result<&Remote, GitError> {
        self.remote.as_ref().ok_or(GitError::NoRemote)
    }

    fn remote_mut(&mut self) -> Result<&mut Remote, GitError> {
        self.remote.as_mut().ok_or(GitError::NoRemote)
    }

    fn branch_at(&self, checkout: &Path) -> Option<BranchName> {
        if checkout == self.main_path.as_path() {
            return self.main_branch.clone();
        }
        self.worktrees
            .iter()
            .find(|worktree| worktree.path.as_path() == checkout && !worktree.folder_deleted)
            .map(|worktree| worktree.branch.clone())
    }

    fn head_at(&self, checkout: &Path) -> Option<&Branch> {
        self.branch_at(checkout)
            .and_then(|name| self.branches.get(name.as_str()))
    }

    /// Where `name` is checked out. Like git, a worktree whose folder was
    /// deleted still counts until it is pruned.
    fn checkout_of(&self, name: &BranchName) -> Option<PathBuf> {
        if self.main_branch.as_ref() == Some(name) {
            return Some(self.main_path.as_path().to_path_buf());
        }
        self.worktrees
            .iter()
            .find(|worktree| &worktree.branch == name)
            .map(|worktree| worktree.path.as_path().to_path_buf())
    }

    fn is_checkout_path(&self, path: &Path) -> bool {
        path == self.main_path.as_path()
            || self
                .worktrees
                .iter()
                .any(|worktree| worktree.path.as_path() == path)
    }

    fn checked_out_error(name: &BranchName, path: PathBuf) -> GitError {
        GitError::BranchCheckedOut {
            name: name.to_string(),
            worktree: WorktreePath::new(path).display(),
        }
    }

    /// The remote branch history `branch` tracks, while that remote branch exists.
    fn upstream_history(&self, branch: &Branch) -> Option<&History> {
        let upstream = branch.upstream.as_ref()?;
        self.remote.as_ref()?.branches.get(upstream.as_str())
    }

    /// Like `git branch -d`: merged into its upstream when it has one,
    /// otherwise into the HEAD of `checkout`.
    fn is_fully_merged(&self, branch: &Branch, checkout: &Path) -> bool {
        match self.upstream_history(branch) {
            Some(upstream) => upstream.contains(branch.tip()),
            None => self
                .head_at(checkout)
                .is_some_and(|head| head.contains(branch.tip())),
        }
    }

    fn tracking_at(&self, checkout: &Path) -> Result<Tracking, GitError> {
        self.remote()?;
        let local = self.head_at(checkout).ok_or(GitError::NoUpstream)?;
        let remote_history = self.upstream_history(local).ok_or(GitError::NoUpstream)?;
        Ok(Tracking {
            local: local.clone(),
            remote_history: remote_history.clone(),
        })
    }

    fn prune(&mut self) {
        self.worktrees.retain(|worktree| !worktree.folder_deleted);
    }
}

struct Failure {
    operation: Operation,
    /// Every repository when `None`.
    repository: Option<PathBuf>,
    error: GitError,
}

#[derive(Default)]
struct State {
    /// Keyed by the path of the main checkout.
    repositories: HashMap<PathBuf, Repository>,
    failures: Vec<Failure>,
    commit_count: usize,
    /// Every `fetch_branch`, keyed by the repository's main checkout, in call
    /// order, including the no-ops: so tests can assert what was fetched.
    fetched_branches: HashMap<PathBuf, Vec<BranchName>>,
}

impl State {
    fn new_commit(&mut self) -> CommitId {
        self.commit_count += 1;
        format!("commit-{}", self.commit_count)
    }

    fn repository(&self, key: &Path) -> &Repository {
        self.repositories
            .get(key)
            .unwrap_or_else(|| panic!("no repository at '{}'", key.display()))
    }

    fn repository_mut(&mut self, key: &Path) -> &mut Repository {
        self.repositories
            .get_mut(key)
            .unwrap_or_else(|| panic!("no repository at '{}'", key.display()))
    }

    /// The repository whose main checkout or live worktree is at `path`.
    fn key_for(&self, path: &Path) -> Option<PathBuf> {
        self.repositories
            .iter()
            .find(|(_, repository)| {
                path == repository.main_path.as_path() || repository.branch_at(path).is_some()
            })
            .map(|(key, _)| key.clone())
    }

    fn failure_for(&self, operation: Operation, key: &Path) -> Option<GitError> {
        self.failures
            .iter()
            .find(|failure| {
                failure.operation == operation
                    && failure
                        .repository
                        .as_deref()
                        .is_none_or(|repository| repository == key)
            })
            .map(|failure| failure.error.clone())
    }
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Default)]
pub struct InMemoryGitRepositories {
    state: Arc<Mutex<State>>,
}

impl InMemoryGitRepositories {
    pub fn new() -> Self {
        Self::default()
    }

    /// A repository with one commit on `default_branch`, checked out in its
    /// main checkout at `path`, and no remote.
    pub fn add_repository(&self, path: &RepositoryPath, default_branch: &BranchName) {
        let mut state = lock(&self.state);
        let first_commit = state.new_commit();
        let branch = Branch::starting_at(default_branch.clone(), vec![first_commit]);
        state.repositories.insert(
            path.as_path().to_path_buf(),
            Repository {
                main_path: path.clone(),
                main_branch: Some(default_branch.clone()),
                branches: BTreeMap::from([(default_branch.to_string(), branch)]),
                worktrees: Vec::new(),
                remote: None,
            },
        );
    }

    /// Configures `origin`, with no branches on it yet.
    pub fn set_remote(&self, repository: &RepositoryPath, url: RemoteUrl) {
        lock(&self.state)
            .repository_mut(repository.as_path())
            .remote = Some(Remote {
            url,
            branches: BTreeMap::new(),
        });
    }

    /// Like `git push -u origin <branch>`: copies the branch to `origin` and
    /// makes it the upstream.
    pub fn publish(&self, repository: &RepositoryPath, branch: &BranchName) {
        let mut state = lock(&self.state);
        let repository = state.repository_mut(repository.as_path());
        let history = repository
            .branch(branch)
            .expect("branch exists")
            .history
            .clone();
        repository
            .remote_mut()
            .expect("a remote is set")
            .branches
            .insert(branch.to_string(), history);
        repository
            .branch_mut(branch)
            .expect("branch exists")
            .upstream = Some(branch.clone());
    }

    /// Deletes the branch on `origin`, leaving a local branch that tracked it
    /// with a gone upstream.
    pub fn delete_remote_branch(&self, repository: &RepositoryPath, branch: &BranchName) {
        lock(&self.state)
            .repository_mut(repository.as_path())
            .remote_mut()
            .expect("a remote is set")
            .branches
            .remove(branch.as_str());
    }

    /// Adds a commit to a local branch.
    pub fn commit(&self, repository: &RepositoryPath, branch: &BranchName) {
        let mut state = lock(&self.state);
        let commit = state.new_commit();
        state
            .repository_mut(repository.as_path())
            .branch_mut(branch)
            .expect("branch exists")
            .history
            .push(commit);
    }

    /// Adds a commit to a branch on `origin`, as if someone else pushed it.
    pub fn commit_on_remote(&self, repository: &RepositoryPath, branch: &BranchName) {
        let mut state = lock(&self.state);
        let commit = state.new_commit();
        state
            .repository_mut(repository.as_path())
            .remote_mut()
            .expect("a remote is set")
            .branches
            .get_mut(branch.as_str())
            .expect("remote branch exists")
            .push(commit);
    }

    /// The worktree's folder disappears from disk; git keeps the worktree
    /// (and its branch checked out) until it is pruned.
    pub fn delete_worktree_folder(&self, repository: &RepositoryPath, path: &WorktreePath) {
        let mut state = lock(&self.state);
        let worktree = state
            .repository_mut(repository.as_path())
            .worktrees
            .iter_mut()
            .find(|worktree| &worktree.path == path)
            .expect("worktree exists");
        worktree.folder_deleted = true;
    }

    /// Whether the branch got commits since it was created, fetched or added:
    /// the notion behind keeping a branch that has work on it.
    pub fn has_commits_beyond_base(
        &self,
        repository: &RepositoryPath,
        branch: &BranchName,
    ) -> bool {
        let state = lock(&self.state);
        let branch = state
            .repository(repository.as_path())
            .branch(branch)
            .expect("branch exists");
        branch.tip() != &branch.base_commit
    }

    /// The branches `fetch_branch` was called with on `repository`, in order,
    /// including the no-ops when the branch already existed locally.
    pub fn fetched_branches(&self, repository: &RepositoryPath) -> Vec<BranchName> {
        lock(&self.state)
            .fetched_branches
            .get(repository.as_path())
            .cloned()
            .unwrap_or_default()
    }

    /// Every later call of `operation`, in any repository, fails with `error`
    /// and changes nothing.
    pub fn fail_on(&self, operation: Operation, error: GitError) {
        lock(&self.state).failures.push(Failure {
            operation,
            repository: None,
            error,
        });
    }

    /// Like `fail_on`, but only in the repository whose main checkout is `repository`.
    pub fn fail_on_in(&self, repository: &RepositoryPath, operation: Operation, error: GitError) {
        lock(&self.state).failures.push(Failure {
            operation,
            repository: Some(repository.as_path().to_path_buf()),
            error,
        });
    }
}

impl GitRepositories for InMemoryGitRepositories {
    fn open(&self, path: &RepositoryPath) -> Result<Box<dyn GitRepository>, GitError> {
        let state = lock(&self.state);
        let key = state
            .key_for(path.as_path())
            .ok_or_else(|| GitError::NotARepository {
                path: path.display(),
            })?;
        if let Some(error) = state.failure_for(Operation::Open, &key) {
            return Err(error);
        }
        Ok(Box::new(InMemoryGitRepository {
            state: self.state.clone(),
            key,
            checkout: path.as_path().to_path_buf(),
        }))
    }

    fn is_repository_root(&self, path: &Path) -> bool {
        lock(&self.state).repositories.contains_key(path)
    }
}

/// A repository opened at `checkout`: its main checkout or one of its worktrees.
struct InMemoryGitRepository {
    state: Arc<Mutex<State>>,
    key: PathBuf,
    checkout: PathBuf,
}

impl InMemoryGitRepository {
    fn read<T>(&self, read: impl FnOnce(&Repository, &Path) -> T) -> T {
        let state = lock(&self.state);
        read(state.repository(&self.key), &self.checkout)
    }

    fn run<T>(
        &self,
        operation: Operation,
        run: impl FnOnce(&mut Repository, &Path) -> Result<T, GitError>,
    ) -> Result<T, GitError> {
        let mut state = lock(&self.state);
        if let Some(error) = state.failure_for(operation, &self.key) {
            return Err(error);
        }
        run(state.repository_mut(&self.key), &self.checkout)
    }
}

impl GitRepository for InMemoryGitRepository {
    fn current_branch(&self) -> Option<BranchName> {
        self.read(|repository, checkout| repository.branch_at(checkout))
    }

    fn has_branch(&self, name: &BranchName) -> bool {
        self.read(|repository, _| repository.branches.contains_key(name.as_str()))
    }

    fn branch_names(&self) -> Result<Vec<BranchName>, GitError> {
        self.run(Operation::BranchNames, |repository, _| {
            Ok(repository
                .branches
                .values()
                .map(|branch| branch.name.clone())
                .collect())
        })
    }

    fn branches(&self) -> Result<Vec<LocalBranch>, GitError> {
        self.run(Operation::Branches, |repository, checkout| {
            let current = repository.branch_at(checkout);
            let head = repository.head_at(checkout);
            Ok(repository
                .branches
                .values()
                .map(|branch| {
                    let upstream_exists = repository.upstream_history(branch).is_some();
                    LocalBranch {
                        name: branch.name.clone(),
                        is_current: current.as_ref() == Some(&branch.name),
                        is_checked_out: repository.checkout_of(&branch.name).is_some(),
                        has_upstream: upstream_exists,
                        upstream_gone: branch.upstream.is_some() && !upstream_exists,
                        is_merged: head.is_some_and(|head| head.contains(branch.tip())),
                    }
                })
                .collect())
        })
    }

    fn worktrees(&self) -> Result<Vec<Checkout>, GitError> {
        self.run(Operation::Worktrees, |repository, _| {
            let main = Checkout {
                path: repository.main_path.clone(),
                branch: repository.main_branch.clone(),
                is_main: true,
            };
            let linked = repository.worktrees.iter().map(|worktree| Checkout {
                path: RepositoryPath::new(worktree.path.as_path()),
                branch: Some(worktree.branch.clone()),
                is_main: false,
            });
            Ok(std::iter::once(main).chain(linked).collect())
        })
    }

    fn worktree_for(&self, branch: &BranchName) -> Result<Option<WorktreePath>, GitError> {
        self.run(Operation::WorktreeFor, |repository, _| {
            Ok(repository.checkout_of(branch).map(WorktreePath::new))
        })
    }

    fn remote_url(&self) -> Option<RemoteUrl> {
        self.read(|repository, _| repository.remote.as_ref().map(|remote| remote.url.clone()))
    }

    fn diff_stats_against(&self, base: Option<&BranchName>) -> Result<DiffStats, GitError> {
        self.run(Operation::DiffStats, |repository, checkout| {
            if let Some(base) = base {
                repository.branch(base)?;
                repository.head_at(checkout).ok_or(GitError::DetachedHead)?;
            }
            Ok(DiffStats::default())
        })
    }

    fn diff_against(&self, base: &BranchName) -> Result<String, GitError> {
        self.run(Operation::Diff, |repository, checkout| {
            repository.branch(base)?;
            repository.head_at(checkout).ok_or(GitError::DetachedHead)?;
            Ok(String::new())
        })
    }

    fn create_branch(&self, name: &BranchName, from: &BranchName) -> Result<(), GitError> {
        self.run(Operation::CreateBranch, |repository, _| {
            if repository.branches.contains_key(name.as_str()) {
                return Err(GitError::BranchAlreadyExists {
                    name: name.to_string(),
                });
            }
            let history = repository.branch(from)?.history.clone();
            repository
                .branches
                .insert(name.to_string(), Branch::starting_at(name.clone(), history));
            Ok(())
        })
    }

    fn delete_branch(&self, name: &BranchName, force: bool) -> Result<(), GitError> {
        self.run(Operation::DeleteBranch, |repository, checkout| {
            let branch = repository.branch(name)?;
            if let Some(path) = repository.checkout_of(name) {
                return Err(Repository::checked_out_error(name, path));
            }
            if !force && !repository.is_fully_merged(branch, checkout) {
                return Err(GitError::CommandFailed {
                    operation: "branch delete".into(),
                    output: format!("error: the branch '{name}' is not fully merged"),
                });
            }
            repository.branches.remove(name.as_str());
            Ok(())
        })
    }

    fn has_commits_beyond(&self, branch: &BranchName, base: &BranchName) -> Result<bool, GitError> {
        self.run(Operation::HasCommitsBeyond, |repository, _| {
            let base_history = &repository.branch(base)?.history;
            Ok(repository
                .branch(branch)?
                .history
                .iter()
                .any(|commit| !base_history.contains(commit)))
        })
    }

    fn add_worktree(
        &self,
        path: &WorktreePath,
        branch: &BranchName,
    ) -> Result<WorktreePath, GitError> {
        self.run(Operation::AddWorktree, |repository, _| {
            repository.branch(branch)?;
            repository.prune();
            if let Some(existing) = repository.checkout_of(branch) {
                return Err(Repository::checked_out_error(branch, existing));
            }
            if repository.is_checkout_path(path.as_path()) {
                return Err(GitError::CommandFailed {
                    operation: "worktree add".into(),
                    output: format!("fatal: '{}' already exists", path.display()),
                });
            }
            repository.worktrees.push(Worktree {
                path: path.clone(),
                branch: branch.clone(),
                folder_deleted: false,
            });
            Ok(path.clone())
        })
    }

    fn remove_worktree(&self, path: &WorktreePath) -> Result<(), GitError> {
        self.run(Operation::RemoveWorktree, |repository, _| {
            let position = repository
                .worktrees
                .iter()
                .position(|worktree| &worktree.path == path)
                .ok_or_else(|| GitError::CommandFailed {
                    operation: "worktree remove".into(),
                    output: format!("fatal: '{}' is not a working tree", path.display()),
                })?;
            repository.worktrees.remove(position);
            repository.prune();
            Ok(())
        })
    }

    fn prune_worktrees(&self) -> Result<(), GitError> {
        self.run(Operation::PruneWorktrees, |repository, _| {
            repository.prune();
            Ok(())
        })
    }

    fn fetch_branch(&self, name: &BranchName) -> Result<(), GitError> {
        let mut state = lock(&self.state);
        if let Some(error) = state.failure_for(Operation::FetchBranch, &self.key) {
            return Err(error);
        }
        state
            .fetched_branches
            .entry(self.key.clone())
            .or_default()
            .push(name.clone());
        let repository = state.repository_mut(&self.key);
        let remote_history = repository.remote()?.branches.get(name.as_str()).cloned();
        if repository.branches.contains_key(name.as_str()) {
            return Ok(());
        }
        let history = remote_history.ok_or_else(|| GitError::CommandFailed {
            operation: "fetch".into(),
            output: format!("fatal: couldn't find remote ref {name}"),
        })?;
        repository
            .branches
            .insert(name.to_string(), Branch::starting_at(name.clone(), history));
        Ok(())
    }

    fn push(&self) -> Result<String, GitError> {
        self.run(Operation::Push, |repository, checkout| {
            repository.remote()?;
            let local = repository.head_at(checkout).ok_or(GitError::DetachedHead)?;
            if repository.upstream_history(local).is_none() {
                let (name, history) = (local.name.clone(), local.history.clone());
                repository
                    .remote_mut()?
                    .branches
                    .insert(name.to_string(), history);
                repository.branch_mut(&name)?.upstream = Some(name.clone());
                return Ok(format!("{name} -> {name} (new branch)"));
            }
            let tracking = repository.tracking_at(checkout)?;
            let remote_tip = tracking
                .remote_history
                .last()
                .expect("a branch has a commit");
            if !tracking.local.contains(remote_tip) {
                return Err(GitError::NotFastForward {
                    operation: "push".into(),
                    output: "! [rejected] (non-fast-forward)".into(),
                });
            }
            let upstream = tracking.local.upstream.clone().expect("tracked branch");
            repository
                .remote_mut()?
                .branches
                .insert(upstream.to_string(), tracking.local.history.clone());
            Ok(format!("{} -> {upstream}", tracking.local.name))
        })
    }

    fn pull(&self) -> Result<String, GitError> {
        self.run(Operation::Pull, |repository, checkout| {
            let tracking = repository.tracking_at(checkout)?;
            let remote_tip = tracking
                .remote_history
                .last()
                .expect("a branch has a commit");
            if tracking.local.contains(remote_tip) {
                return Ok("Already up to date.".into());
            }
            if !tracking.remote_history.contains(tracking.local.tip()) {
                return Err(GitError::NotFastForward {
                    operation: "pull".into(),
                    output: "fatal: Not possible to fast-forward, aborting.".into(),
                });
            }
            repository.branch_mut(&tracking.local.name)?.history = tracking.remote_history;
            Ok("Fast-forward".into())
        })
    }
}

#[path = "in_memory_tests.rs"]
mod tests;
