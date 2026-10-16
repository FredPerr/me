//! The git adapter: git2 for reads and branch creation, the `git` command line
//! for worktrees, branch deletion and network operations.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use git2::{BranchType, Delta, DiffFormat, Repository, Status, StatusOptions};

use super::git_command::{self, GitOutput};
use super::repository_lock;
use crate::git::domain::branch_name::BranchName;
use crate::git::domain::error::GitError;
use crate::git::domain::paths::{RepositoryPath, WorktreePath};
use crate::git::domain::ports::{
    Checkout, DiffStats, GitRepositories, GitRepository, LocalBranch, RemoteAuth, RemoteCredentials,
};
use crate::git::domain::remote_url::RemoteUrl;

const ORIGIN: &str = "origin";
const HEADS_PREFIX: &str = "refs/heads/";
const COMMON_DIR_FILE: &str = "commondir";
/// A very large diff is of little use in an agent prompt and risks exhausting
/// memory, so it is cut here with a trailing marker.
const MAX_DIFF_BYTES: usize = 200_000;
const DIFF_TRUNCATED_MARKER: &str = "\n\n[diff truncated: too large to include in full]\n";

pub struct Git2CliRepositories {
    credentials: Arc<dyn RemoteCredentials>,
}

impl Git2CliRepositories {
    pub fn new(credentials: Arc<dyn RemoteCredentials>) -> Self {
        Self { credentials }
    }
}

impl GitRepositories for Git2CliRepositories {
    fn open(&self, path: &RepositoryPath) -> Result<Box<dyn GitRepository>, GitError> {
        let repository =
            Repository::open(path.as_path()).map_err(|_| GitError::NotARepository {
                path: path.display(),
            })?;
        let lock = repository_lock::for_common_dir(&common_dir(&repository));
        Ok(Box::new(Git2CliRepository {
            path: path.clone(),
            repository,
            lock,
            credentials: self.credentials.clone(),
        }))
    }

    fn is_repository_root(&self, path: &Path) -> bool {
        path.join(".git").exists()
    }
}

struct Git2CliRepository {
    path: RepositoryPath,
    repository: Repository,
    lock: Arc<Mutex<()>>,
    credentials: Arc<dyn RemoteCredentials>,
}

/// The upstream a branch tracks, split into remote and remote branch names.
struct Upstream {
    remote: String,
    remote_branch: String,
}

impl Git2CliRepository {
    fn write_guard(&self) -> MutexGuard<'_, ()> {
        self.lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn git(&self, operation: &str, args: &[&str]) -> Result<GitOutput, GitError> {
        git_command::run(self.path.as_path(), operation, args, None)
    }

    fn git_with_auth(
        &self,
        operation: &str,
        args: &[&str],
        auth: Option<&RemoteAuth>,
    ) -> Result<GitOutput, GitError> {
        git_command::run(self.path.as_path(), operation, args, auth)
    }

    fn auth_for_remote(&self, remote_name: &str) -> Option<RemoteAuth> {
        let remote = self.repository.find_remote(remote_name).ok()?;
        let url = RemoteUrl::new(remote.url()?);
        self.credentials.for_remote(&url)
    }

    fn require_origin(&self) -> Result<(), GitError> {
        self.repository
            .find_remote(ORIGIN)
            .map(|_| ())
            .map_err(|_| GitError::NoRemote)
    }

    fn upstream(&self) -> Option<Upstream> {
        let head = self.repository.head().ok()?;
        if !head.is_branch() {
            return None;
        }
        let branch_name = head.shorthand()?;
        let branch = self
            .repository
            .find_branch(branch_name, BranchType::Local)
            .ok()?;
        let upstream = branch.upstream().ok()?;
        // Resolved through the branch's configured remote so slashes in either
        // name stay correct ("origin/main" is ambiguous on its own).
        let remote = self
            .repository
            .branch_upstream_remote(&format!("{HEADS_PREFIX}{branch_name}"))
            .ok()?;
        let remote = remote.as_str()?.to_string();
        let upstream_ref = upstream.get().name()?;
        let remote_branch = upstream_ref
            .strip_prefix(&format!("refs/remotes/{remote}/"))?
            .to_string();
        Some(Upstream {
            remote,
            remote_branch,
        })
    }

    fn checked_out_branch_names(&self) -> Result<HashSet<String>, GitError> {
        Ok(self
            .worktrees()?
            .into_iter()
            .filter_map(|checkout| checkout.branch)
            .map(|branch| branch.as_str().to_string())
            .collect())
    }

    fn branch_has_upstream_config(&self, name: &str) -> bool {
        self.repository
            .config()
            .and_then(|config| config.get_string(&format!("branch.{name}.merge")))
            .is_ok()
    }

    fn resolve_commit(&self, name: &BranchName) -> Result<git2::Commit<'_>, GitError> {
        let not_found = || GitError::BranchNotFound {
            name: name.to_string(),
        };
        for branch_type in [BranchType::Local, BranchType::Remote] {
            if let Ok(branch) = self.repository.find_branch(name.as_str(), branch_type) {
                return branch.get().peel_to_commit().map_err(|_| not_found());
            }
        }
        self.repository
            .find_reference(&format!("refs/remotes/{name}"))
            .and_then(|reference| reference.peel_to_commit())
            .map_err(|_| not_found())
    }

    fn head_commit(&self) -> Result<git2::Commit<'_>, GitError> {
        self.repository
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(|_| GitError::DetachedHead)
    }

    fn diff_since_merge_base(&self, base: &BranchName) -> Result<git2::Diff<'_>, GitError> {
        let base_commit = self.resolve_commit(base)?;
        let head_commit = self.head_commit()?;
        let failed = |error: git2::Error| GitError::CommandFailed {
            operation: "diff".into(),
            output: error.message().to_string(),
        };
        let merge_base = self
            .repository
            .merge_base(base_commit.id(), head_commit.id())
            .map_err(failed)?;
        let merge_base_tree = self
            .repository
            .find_commit(merge_base)
            .and_then(|commit| commit.tree())
            .map_err(failed)?;
        let head_tree = head_commit.tree().map_err(failed)?;
        self.repository
            .diff_tree_to_tree(Some(&merge_base_tree), Some(&head_tree), None)
            .map_err(failed)
    }

    fn uncommitted_diff_stats(&self) -> Result<DiffStats, GitError> {
        let mut options = StatusOptions::new();
        options.include_untracked(true);
        options.exclude_submodules(true);
        let statuses = self
            .repository
            .statuses(Some(&mut options))
            .map_err(|error| GitError::io("read the status", error))?;

        let mut stats = DiffStats::default();
        for entry in statuses.iter() {
            let status = entry.status();
            if status.intersects(Status::WT_NEW | Status::INDEX_NEW) {
                stats.files_added += 1;
            } else if status.intersects(Status::WT_DELETED | Status::INDEX_DELETED) {
                stats.files_deleted += 1;
            } else if status.intersects(Status::WT_RENAMED | Status::INDEX_RENAMED) {
                stats.files_renamed += 1;
            } else if status.intersects(
                Status::WT_MODIFIED
                    | Status::INDEX_MODIFIED
                    | Status::WT_TYPECHANGE
                    | Status::INDEX_TYPECHANGE,
            ) {
                stats.files_modified += 1;
            }
        }

        if let Ok(diff) = self.repository.diff_index_to_workdir(None, None) {
            add_line_counts(&mut stats, &diff);
        }
        if let Ok(tree) = self.repository.head().and_then(|head| head.peel_to_tree()) {
            if let Ok(diff) = self.repository.diff_tree_to_index(Some(&tree), None, None) {
                add_line_counts(&mut stats, &diff);
            }
        }
        Ok(stats)
    }

    fn worktree_for_unlocked(&self, branch: &BranchName) -> Result<Option<WorktreePath>, GitError> {
        let listing = self.git("worktree list", &["worktree", "list", "--porcelain"])?;
        let target_ref = format!("{HEADS_PREFIX}{branch}");
        let mut current_path: Option<&str> = None;
        for line in listing.stdout.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                current_path = Some(path);
            } else if line.strip_prefix("branch ") == Some(target_ref.as_str()) {
                return Ok(current_path.map(WorktreePath::new));
            }
        }
        Ok(None)
    }

    fn prune_unlocked(&self) -> Result<(), GitError> {
        self.git("worktree prune", &["worktree", "prune"])
            .map(|_| ())
    }
}

/// The git folder shared by all worktrees of the repository. A linked
/// worktree's own git folder holds a `commondir` file pointing at it.
fn common_dir(repository: &Repository) -> PathBuf {
    let git_dir = repository.path();
    std::fs::read_to_string(git_dir.join(COMMON_DIR_FILE))
        .map(|relative| git_dir.join(relative.trim()))
        .unwrap_or_else(|_| git_dir.to_path_buf())
}

fn add_line_counts(stats: &mut DiffStats, diff: &git2::Diff<'_>) {
    if let Ok(diff_stats) = diff.stats() {
        stats.insertions += diff_stats.insertions();
        stats.deletions += diff_stats.deletions();
    }
}

fn head_branch_of(repository: &Repository) -> Option<BranchName> {
    let head = repository.head().ok()?;
    BranchName::parse(head.shorthand()?).ok()
}

fn truncate_diff(mut diff: String) -> String {
    if diff.len() <= MAX_DIFF_BYTES {
        return diff;
    }
    let mut end = MAX_DIFF_BYTES;
    while !diff.is_char_boundary(end) {
        end -= 1;
    }
    diff.truncate(end);
    diff.push_str(DIFF_TRUNCATED_MARKER);
    diff
}

impl GitRepository for Git2CliRepository {
    fn current_branch(&self) -> Option<BranchName> {
        head_branch_of(&self.repository)
    }

    fn has_branch(&self, name: &BranchName) -> bool {
        self.repository
            .find_branch(name.as_str(), BranchType::Local)
            .is_ok()
    }

    fn branch_names(&self) -> Result<Vec<BranchName>, GitError> {
        let branches = self
            .repository
            .branches(Some(BranchType::Local))
            .map_err(|error| GitError::io("list branches", error))?;
        let mut names: Vec<BranchName> = branches
            .filter_map(Result::ok)
            .filter_map(|(branch, _)| branch.name().ok().flatten().map(str::to_string))
            .filter_map(|name| BranchName::parse(&name).ok())
            .collect();
        names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok(names)
    }

    fn branches(&self) -> Result<Vec<LocalBranch>, GitError> {
        let checked_out = self.checked_out_branch_names()?;
        let current = self.current_branch();
        let head_commit = self
            .repository
            .head()
            .ok()
            .and_then(|head| head.peel_to_commit().ok());
        let branches = self
            .repository
            .branches(Some(BranchType::Local))
            .map_err(|error| GitError::io("list branches", error))?;

        let mut local_branches = Vec::new();
        for (branch, _) in branches.filter_map(Result::ok) {
            let Some(name) = branch.name().ok().flatten().map(str::to_string) else {
                continue;
            };
            let Ok(branch_name) = BranchName::parse(&name) else {
                continue;
            };
            let upstream = branch.upstream();
            // git2 reports NotFound for a configured upstream whose ref is gone.
            let upstream_gone = matches!(&upstream, Err(error) if error.code() == git2::ErrorCode::NotFound)
                && self.branch_has_upstream_config(&name);
            let is_merged = match (&head_commit, branch.get().peel_to_commit().ok()) {
                (Some(head), Some(commit)) => {
                    head.id() == commit.id()
                        || self
                            .repository
                            .graph_descendant_of(head.id(), commit.id())
                            .unwrap_or(false)
                }
                _ => false,
            };
            local_branches.push(LocalBranch {
                is_current: current.as_ref() == Some(&branch_name),
                is_checked_out: checked_out.contains(&name),
                has_upstream: upstream.is_ok(),
                upstream_gone,
                is_merged,
                name: branch_name,
            });
        }
        local_branches.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        Ok(local_branches)
    }

    fn worktrees(&self) -> Result<Vec<Checkout>, GitError> {
        let main_path = self
            .repository
            .workdir()
            .map(RepositoryPath::new)
            .unwrap_or_else(|| self.path.clone());
        let mut checkouts = vec![Checkout {
            path: main_path,
            branch: self.current_branch(),
            is_main: true,
        }];
        let names = self
            .repository
            .worktrees()
            .map_err(|error| GitError::io("list worktrees", error))?;
        for name in names.iter().flatten() {
            let Ok(worktree) = self.repository.find_worktree(name) else {
                continue;
            };
            let branch = Repository::open(worktree.path())
                .ok()
                .and_then(|worktree_repository| head_branch_of(&worktree_repository));
            checkouts.push(Checkout {
                path: RepositoryPath::new(worktree.path()),
                branch,
                is_main: false,
            });
        }
        Ok(checkouts)
    }

    fn worktree_for(&self, branch: &BranchName) -> Result<Option<WorktreePath>, GitError> {
        self.worktree_for_unlocked(branch)
    }

    fn remote_url(&self) -> Option<RemoteUrl> {
        let remote = self.repository.find_remote(ORIGIN).ok()?;
        remote.url().map(RemoteUrl::new)
    }

    fn diff_stats_against(&self, base: Option<&BranchName>) -> Result<DiffStats, GitError> {
        let Some(base) = base else {
            return self.uncommitted_diff_stats();
        };
        let diff = self.diff_since_merge_base(base)?;
        let mut stats = DiffStats::default();
        for delta in diff.deltas() {
            match delta.status() {
                Delta::Added => stats.files_added += 1,
                Delta::Deleted => stats.files_deleted += 1,
                Delta::Modified => stats.files_modified += 1,
                Delta::Renamed => stats.files_renamed += 1,
                _ => {}
            }
        }
        add_line_counts(&mut stats, &diff);
        Ok(stats)
    }

    fn diff_against(&self, base: &BranchName) -> Result<String, GitError> {
        let diff = self.diff_since_merge_base(base)?;
        let mut patch = String::new();
        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            if matches!(line.origin(), '+' | '-' | ' ') {
                patch.push(line.origin());
            }
            patch.push_str(&String::from_utf8_lossy(line.content()));
            true
        })
        .map_err(|error| GitError::io("print the diff", error))?;
        Ok(truncate_diff(patch))
    }

    fn create_branch(&self, name: &BranchName, from: &BranchName) -> Result<(), GitError> {
        let _guard = self.write_guard();
        if self.has_branch(name) {
            return Err(GitError::BranchAlreadyExists {
                name: name.to_string(),
            });
        }
        let base_commit = self
            .repository
            .find_branch(from.as_str(), BranchType::Local)
            .and_then(|branch| branch.get().peel_to_commit())
            .map_err(|_| GitError::BranchNotFound {
                name: from.to_string(),
            })?;
        self.repository
            .branch(name.as_str(), &base_commit, false)
            .map(|_| ())
            .map_err(|error| GitError::io(format!("create branch '{name}'"), error))
    }

    fn delete_branch(&self, name: &BranchName, force: bool) -> Result<(), GitError> {
        let _guard = self.write_guard();
        if !self.has_branch(name) {
            return Err(GitError::BranchNotFound {
                name: name.to_string(),
            });
        }
        if let Some(worktree) = self.worktree_for_unlocked(name)? {
            return Err(GitError::BranchCheckedOut {
                name: name.to_string(),
                worktree: worktree.display(),
            });
        }
        let flag = if force { "-D" } else { "-d" };
        self.git("branch delete", &["branch", flag, name.as_str()])
            .map(|_| ())
    }

    fn has_commits_beyond(&self, branch: &BranchName, base: &BranchName) -> Result<bool, GitError> {
        let branch_commit = self.resolve_commit(branch)?.id();
        let base_commit = self.resolve_commit(base)?.id();
        let (ahead, _behind) = self
            .repository
            .graph_ahead_behind(branch_commit, base_commit)
            .map_err(|error| GitError::io(format!("compare '{branch}' with '{base}'"), error))?;
        Ok(ahead > 0)
    }

    fn add_worktree(
        &self,
        path: &WorktreePath,
        branch: &BranchName,
    ) -> Result<WorktreePath, GitError> {
        let _guard = self.write_guard();
        if !self.has_branch(branch) {
            return Err(GitError::BranchNotFound {
                name: branch.to_string(),
            });
        }
        self.prune_unlocked()?;
        if let Some(existing) = self.worktree_for_unlocked(branch)? {
            return Err(GitError::BranchCheckedOut {
                name: branch.to_string(),
                worktree: existing.display(),
            });
        }
        let target = path.as_path().to_string_lossy().to_string();
        self.git(
            "worktree add",
            &["worktree", "add", &target, branch.as_str()],
        )?;
        Ok(path.clone())
    }

    fn remove_worktree(&self, path: &WorktreePath) -> Result<(), GitError> {
        let _guard = self.write_guard();
        let target = path.as_path().to_string_lossy().to_string();
        self.git(
            "worktree remove",
            &["worktree", "remove", "--force", &target],
        )?;
        if path.as_path().exists() {
            std::fs::remove_dir_all(path.as_path())
                .map_err(|error| GitError::io(format!("delete '{}'", path.display()), error))?;
        }
        self.prune_unlocked()
    }

    fn prune_worktrees(&self) -> Result<(), GitError> {
        let _guard = self.write_guard();
        self.prune_unlocked()
    }

    fn fetch_branch(&self, name: &BranchName) -> Result<(), GitError> {
        self.require_origin()?;
        if self.has_branch(name) {
            return Ok(());
        }
        let _guard = self.write_guard();
        let refspec = format!("{name}:{HEADS_PREFIX}{name}");
        let auth = self.auth_for_remote(ORIGIN);
        self.git_with_auth("fetch", &["fetch", ORIGIN, &refspec], auth.as_ref())
            .map(|_| ())
    }

    fn push(&self) -> Result<String, GitError> {
        self.require_origin()?;
        let local_branch = self.current_branch().ok_or(GitError::DetachedHead)?;
        let _guard = self.write_guard();
        let output = match self.upstream() {
            Some(upstream) => {
                let refspec = format!("{local_branch}:{}", upstream.remote_branch);
                let auth = self.auth_for_remote(&upstream.remote);
                self.git_with_auth("push", &["push", &upstream.remote, &refspec], auth.as_ref())?
            }
            None => {
                let auth = self.auth_for_remote(ORIGIN);
                self.git_with_auth(
                    "push",
                    &["push", "--set-upstream", ORIGIN, local_branch.as_str()],
                    auth.as_ref(),
                )?
            }
        };
        Ok(output.stderr)
    }

    fn pull(&self) -> Result<String, GitError> {
        self.require_origin()?;
        let upstream = self.upstream().ok_or(GitError::NoUpstream)?;
        let _guard = self.write_guard();
        let auth = self.auth_for_remote(&upstream.remote);
        // Fetch only the tracked branch so FETCH_HEAD holds exactly one entry,
        // then fast-forward to it.
        self.git_with_auth(
            "fetch",
            &["fetch", &upstream.remote, &upstream.remote_branch],
            auth.as_ref(),
        )?;
        let merge = self.git("pull", &["merge", "--ff-only", "FETCH_HEAD"])?;
        Ok(merge.stdout)
    }
}

#[cfg(test)]
#[path = "git2_cli_tests.rs"]
mod tests;
