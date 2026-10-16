//! Tests of the git adapter against real repositories in temporary folders. A
//! local bare repository stands in for the remote, so nothing touches the network.
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use super::*;

const MAIN: &str = "main";
const FEATURE: &str = "feature/login";
const README: &str = "README.md";
const MISSING: &str = "missing";
const MERGED: &str = "merged";
const ALPHA: &str = "alpha";
const ZETA: &str = "zeta";
const LOGIN_CHECKOUT: &str = "login/repo";
const LOGIN_FILE: &str = "login.txt";
const FIRST_CHECKOUT: &str = "first/repo";
const SECOND_CHECKOUT: &str = "second/repo";

struct NoCredentials;

impl RemoteCredentials for NoCredentials {
    fn for_remote(&self, _remote_url: &RemoteUrl) -> Option<RemoteAuth> {
        None
    }
}

/// Runs git with a fixed identity and no signing, independent of the user's config.
fn git(folder: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(folder)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn commit_file(folder: &Path, file: &str, content: &str) {
    std::fs::write(folder.join(file), content).unwrap();
    git(folder, &["add", file]);
    git(folder, &["commit", "-m", &format!("Change {file}")]);
}

fn name(raw: &str) -> BranchName {
    BranchName::parse(raw).unwrap()
}

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    main_checkout: PathBuf,
    remote: PathBuf,
}

impl Fixture {
    /// A repository on `main` with one commit, pushed to a bare `origin`.
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let root = std::fs::canonicalize(temp.path()).unwrap();
        let remote = root.join("remote.git");
        let main_checkout = root.join("repo");
        std::fs::create_dir_all(&main_checkout).unwrap();
        git(&root, &["init", "--bare", "-b", MAIN, "remote.git"]);
        git(&main_checkout, &["init", "-b", MAIN]);
        commit_file(&main_checkout, README, "hello\n");
        git(
            &main_checkout,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        git(&main_checkout, &["push", "-u", "origin", MAIN]);
        Self {
            _temp: temp,
            root,
            main_checkout,
            remote,
        }
    }

    fn open(&self, path: &Path) -> Box<dyn GitRepository> {
        Git2CliRepositories::new(Arc::new(NoCredentials))
            .open(&RepositoryPath::new(path))
            .unwrap()
    }

    fn main_repository(&self) -> Box<dyn GitRepository> {
        self.open(&self.main_checkout)
    }

    fn worktree_path(&self, folder: &str) -> WorktreePath {
        WorktreePath::new(self.root.join(".worktrees").join(folder))
    }

    /// A second clone that can push commits to `origin` behind the main checkout's back.
    fn other_clone(&self) -> PathBuf {
        let clone = self.root.join("other");
        git(
            &self.root,
            &["clone", self.remote.to_str().unwrap(), "other"],
        );
        clone
    }
}

#[test]
fn opening_a_folder_that_is_not_a_repository_fails() {
    let temp = TempDir::new().unwrap();
    let result =
        Git2CliRepositories::new(Arc::new(NoCredentials)).open(&RepositoryPath::new(temp.path()));
    assert!(matches!(result, Err(GitError::NotARepository { .. })));
}

#[test]
fn recognizes_a_repository_root() {
    let fixture = Fixture::new();
    let repositories = Git2CliRepositories::new(Arc::new(NoCredentials));
    assert!(repositories.is_repository_root(&fixture.main_checkout));
    assert!(!repositories.is_repository_root(&fixture.root));
}

#[test]
fn creates_a_branch_from_its_base() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();

    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();

    assert!(repository.has_branch(&name(FEATURE)));
    assert_eq!(
        git(&fixture.main_checkout, &["rev-parse", FEATURE]),
        git(&fixture.main_checkout, &["rev-parse", MAIN])
    );
}

#[test]
fn refuses_to_create_a_branch_twice_or_from_a_missing_base() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();

    assert_eq!(
        repository.create_branch(&name(FEATURE), &name(MAIN)),
        Err(GitError::BranchAlreadyExists {
            name: FEATURE.into()
        })
    );
    assert_eq!(
        repository.create_branch(&name("other"), &name(MISSING)),
        Err(GitError::BranchNotFound {
            name: MISSING.into()
        })
    );
}

#[test]
fn lists_branch_names_sorted() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository.create_branch(&name(ZETA), &name(MAIN)).unwrap();
    repository.create_branch(&name(ALPHA), &name(MAIN)).unwrap();

    let names: Vec<String> = repository
        .branch_names()
        .unwrap()
        .into_iter()
        .map(|branch| branch.to_string())
        .collect();

    assert_eq!(names, [ALPHA, MAIN, ZETA]);
}

#[test]
fn adds_a_worktree_checked_out_on_the_branch() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    let target = fixture.worktree_path(LOGIN_CHECKOUT);

    let worktree = repository.add_worktree(&target, &name(FEATURE)).unwrap();

    assert_eq!(worktree, target);
    assert!(target.as_path().join(README).is_file());
    assert_eq!(
        fixture.open(target.as_path()).current_branch(),
        Some(name(FEATURE))
    );
}

#[test]
fn refuses_a_worktree_for_a_branch_checked_out_in_another_worktree() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    let first = repository
        .add_worktree(&fixture.worktree_path(FIRST_CHECKOUT), &name(FEATURE))
        .unwrap();
    let second = fixture.worktree_path(SECOND_CHECKOUT);

    let result = repository.add_worktree(&second, &name(FEATURE));

    assert_eq!(
        result,
        Err(GitError::BranchCheckedOut {
            name: FEATURE.into(),
            worktree: first.display()
        })
    );
    assert!(!second.as_path().exists());
}

#[test]
fn refuses_a_worktree_for_the_branch_of_the_main_checkout() {
    let fixture = Fixture::new();

    let result = fixture
        .main_repository()
        .add_worktree(&fixture.worktree_path(FIRST_CHECKOUT), &name(MAIN));

    assert_eq!(
        result,
        Err(GitError::BranchCheckedOut {
            name: MAIN.into(),
            worktree: fixture.main_checkout.to_string_lossy().into_owned()
        })
    );
}

#[test]
fn checks_out_a_branch_whose_previous_worktree_folder_was_deleted() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    let first = fixture.worktree_path(FIRST_CHECKOUT);
    repository.add_worktree(&first, &name(FEATURE)).unwrap();
    std::fs::remove_dir_all(first.as_path()).unwrap();
    let second = fixture.worktree_path(SECOND_CHECKOUT);

    let worktree = repository.add_worktree(&second, &name(FEATURE)).unwrap();

    assert_eq!(worktree, second);
    assert!(second.as_path().join(README).is_file());
}

#[test]
fn reports_where_a_branch_is_checked_out() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    let target = fixture.worktree_path(LOGIN_CHECKOUT);
    repository.add_worktree(&target, &name(FEATURE)).unwrap();

    assert_eq!(
        repository
            .worktree_for(&name(FEATURE))
            .unwrap()
            .map(|path| path.display()),
        Some(target.display())
    );
    assert_eq!(repository.worktree_for(&name("unknown")).unwrap(), None);
}

#[test]
fn refuses_a_worktree_for_a_missing_branch() {
    let fixture = Fixture::new();
    let result = fixture
        .main_repository()
        .add_worktree(&fixture.worktree_path("x/repo"), &name(MISSING));
    assert_eq!(
        result,
        Err(GitError::BranchNotFound {
            name: MISSING.into()
        })
    );
}

#[test]
fn lists_the_main_checkout_then_worktrees() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    let target = fixture.worktree_path(LOGIN_CHECKOUT);
    repository.add_worktree(&target, &name(FEATURE)).unwrap();

    let checkouts = repository.worktrees().unwrap();

    assert_eq!(checkouts.len(), 2);
    assert!(checkouts[0].is_main);
    assert_eq!(
        checkouts[0].path.display(),
        fixture.main_checkout.to_string_lossy()
    );
    assert_eq!(checkouts[0].branch, Some(name(MAIN)));
    assert!(!checkouts[1].is_main);
    assert_eq!(checkouts[1].branch, Some(name(FEATURE)));
}

#[test]
fn removes_a_worktree_and_its_folder() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    let target = fixture.worktree_path(LOGIN_CHECKOUT);
    repository.add_worktree(&target, &name(FEATURE)).unwrap();

    repository.remove_worktree(&target).unwrap();

    assert!(!target.as_path().exists());
    assert_eq!(repository.worktrees().unwrap().len(), 1);
}

#[test]
fn pruning_frees_a_branch_whose_worktree_folder_was_deleted() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    let target = fixture.worktree_path(LOGIN_CHECKOUT);
    repository.add_worktree(&target, &name(FEATURE)).unwrap();
    std::fs::remove_dir_all(target.as_path()).unwrap();

    repository.prune_worktrees().unwrap();

    assert_eq!(repository.worktree_for(&name(FEATURE)).unwrap(), None);
}

#[test]
fn describes_local_branches() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(MERGED), &name(MAIN))
        .unwrap();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    let target = fixture.worktree_path(LOGIN_CHECKOUT);
    repository.add_worktree(&target, &name(FEATURE)).unwrap();
    commit_file(target.as_path(), LOGIN_FILE, "login\n");

    let branches = repository.branches().unwrap();
    let by_name = |raw: &str| branches.iter().find(|b| b.name.as_str() == raw).unwrap();

    let main = by_name(MAIN);
    assert!(main.is_current && main.is_checked_out && main.has_upstream && main.is_merged);
    let feature = by_name(FEATURE);
    assert!(!feature.is_current && feature.is_checked_out);
    assert!(!feature.is_merged && !feature.has_upstream);
    let merged = by_name(MERGED);
    assert!(merged.is_merged && !merged.is_checked_out);
}

#[test]
fn flags_a_branch_whose_upstream_was_deleted() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    git(&fixture.main_checkout, &["push", "-u", "origin", FEATURE]);
    git(
        &fixture.main_checkout,
        &["push", "origin", "--delete", FEATURE],
    );

    let branches = repository.branches().unwrap();
    let feature = branches
        .iter()
        .find(|b| b.name.as_str() == FEATURE)
        .unwrap();

    assert!(feature.upstream_gone);
}

#[test]
fn deletes_merged_branches_and_needs_force_for_unmerged_ones() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(MERGED), &name(MAIN))
        .unwrap();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    let target = fixture.worktree_path(LOGIN_CHECKOUT);
    repository.add_worktree(&target, &name(FEATURE)).unwrap();
    commit_file(target.as_path(), LOGIN_FILE, "login\n");
    repository.remove_worktree(&target).unwrap();

    repository.delete_branch(&name(MERGED), false).unwrap();
    let unforced = repository.delete_branch(&name(FEATURE), false);
    repository.delete_branch(&name(FEATURE), true).unwrap();

    assert!(matches!(unforced, Err(GitError::CommandFailed { .. })));
    assert!(!repository.has_branch(&name(MERGED)));
    assert!(!repository.has_branch(&name(FEATURE)));
}

#[test]
fn refuses_to_delete_a_checked_out_branch() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    let target = fixture.worktree_path(LOGIN_CHECKOUT);
    repository.add_worktree(&target, &name(FEATURE)).unwrap();

    let result = repository.delete_branch(&name(FEATURE), true);

    assert_eq!(
        result,
        Err(GitError::BranchCheckedOut {
            name: FEATURE.into(),
            worktree: target.display()
        })
    );
}

#[test]
fn fetches_a_remote_branch_into_a_local_one() {
    let fixture = Fixture::new();
    let other = fixture.other_clone();
    git(&other, &["checkout", "-b", FEATURE]);
    commit_file(&other, LOGIN_FILE, "login\n");
    git(&other, &["push", "origin", FEATURE]);
    let repository = fixture.main_repository();

    repository.fetch_branch(&name(FEATURE)).unwrap();

    assert!(repository.has_branch(&name(FEATURE)));
    assert_eq!(
        git(&fixture.main_checkout, &["rev-parse", FEATURE]),
        git(&other, &["rev-parse", "HEAD"])
    );
}

#[test]
fn fetching_a_branch_that_exists_locally_does_nothing() {
    let fixture = Fixture::new();
    let repository = fixture.main_repository();
    repository
        .create_branch(&name(FEATURE), &name(MAIN))
        .unwrap();
    std::fs::remove_dir_all(&fixture.remote).unwrap();

    assert_eq!(repository.fetch_branch(&name(FEATURE)), Ok(()));
}

#[test]
fn network_operations_need_an_origin_remote() {
    let fixture = Fixture::new();
    git(&fixture.main_checkout, &["remote", "remove", "origin"]);
    let repository = fixture.main_repository();

    assert_eq!(
        repository.fetch_branch(&name(FEATURE)),
        Err(GitError::NoRemote)
    );
    assert_eq!(repository.pull(), Err(GitError::NoRemote));
    assert_eq!(repository.push(), Err(GitError::NoRemote));
    assert_eq!(repository.remote_url(), None);
}

#[test]
fn pull_fast_forwards_to_the_upstream() {
    let fixture = Fixture::new();
    let other = fixture.other_clone();
    commit_file(&other, "news.txt", "news\n");
    git(&other, &["push", "origin", MAIN]);

    fixture.main_repository().pull().unwrap();

    assert!(fixture.main_checkout.join("news.txt").is_file());
}

#[test]
fn pull_refuses_to_merge_a_diverged_branch() {
    let fixture = Fixture::new();
    let other = fixture.other_clone();
    commit_file(&other, "theirs.txt", "theirs\n");
    git(&other, &["push", "origin", MAIN]);
    commit_file(&fixture.main_checkout, "ours.txt", "ours\n");

    let result = fixture.main_repository().pull();

    assert!(matches!(result, Err(GitError::NotFastForward { .. })));
}

#[test]
fn push_sends_commits_to_the_upstream() {
    let fixture = Fixture::new();
    commit_file(&fixture.main_checkout, "ours.txt", "ours\n");

    fixture.main_repository().push().unwrap();

    assert_eq!(
        git(&fixture.remote, &["rev-parse", MAIN]),
        git(&fixture.main_checkout, &["rev-parse", MAIN])
    );
}

#[test]
fn push_and_pull_need_an_upstream() {
    let fixture = Fixture::new();
    git(&fixture.main_checkout, &["checkout", "-b", FEATURE]);
    let repository = fixture.main_repository();

    assert_eq!(repository.push(), Err(GitError::NoUpstream));
    assert_eq!(repository.pull(), Err(GitError::NoUpstream));
}

#[test]
fn counts_changes_since_the_base_branch() {
    let fixture = Fixture::new();
    git(&fixture.main_checkout, &["checkout", "-b", FEATURE]);
    commit_file(&fixture.main_checkout, LOGIN_FILE, "one\ntwo\n");
    commit_file(&fixture.main_checkout, README, "hello\nworld\n");

    let stats = fixture
        .main_repository()
        .diff_stats_against(Some(&name(MAIN)))
        .unwrap();

    assert_eq!(
        stats,
        DiffStats {
            files_added: 1,
            files_modified: 1,
            insertions: 3,
            ..DiffStats::default()
        }
    );
}

#[test]
fn counts_uncommitted_changes_without_a_base() {
    let fixture = Fixture::new();
    std::fs::write(fixture.main_checkout.join(README), "changed\n").unwrap();
    std::fs::write(fixture.main_checkout.join("new.txt"), "new\n").unwrap();

    let stats = fixture.main_repository().diff_stats_against(None).unwrap();

    assert_eq!(stats.files_added, 1);
    assert_eq!(stats.files_modified, 1);
    assert_eq!((stats.insertions, stats.deletions), (1, 1));
}

#[test]
fn diffs_against_the_base_branch() {
    let fixture = Fixture::new();
    git(&fixture.main_checkout, &["checkout", "-b", FEATURE]);
    commit_file(&fixture.main_checkout, LOGIN_FILE, "login form\n");

    let diff = fixture.main_repository().diff_against(&name(MAIN)).unwrap();

    assert!(diff.contains(LOGIN_FILE));
    assert!(diff.contains("+login form"));
}

#[test]
fn truncates_very_large_diffs() {
    let large = "x".repeat(MAX_DIFF_BYTES + 10);

    let truncated = truncate_diff(large);

    assert!(truncated.ends_with(DIFF_TRUNCATED_MARKER));
    assert_eq!(
        truncated.len(),
        MAX_DIFF_BYTES + DIFF_TRUNCATED_MARKER.len()
    );
}

#[test]
fn reports_the_origin_remote() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.main_repository().remote_url(),
        Some(RemoteUrl::new(fixture.remote.to_string_lossy()))
    );
}
