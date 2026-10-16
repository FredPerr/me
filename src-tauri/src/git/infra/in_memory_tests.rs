//! Tests of the fake's own behaviour, mirroring the git adapter's tests so the
//! fake can be trusted in tests of code that uses git.
use super::*;

const API: &str = "/work/api";
const WEB: &str = "/work/web";
const UNKNOWN: &str = "/work/unknown";
const LOGIN_CHECKOUT: &str = "/work/.worktrees/login/api";
const OTHER_CHECKOUT: &str = "/work/.worktrees/other/api";
const MAIN: &str = "main";
const FEATURE: &str = "feature/login";
const MERGED: &str = "merged";
const ALPHA: &str = "alpha";
const ZETA: &str = "zeta";
const MISSING: &str = "missing";
const ORIGIN_URL: &str = "git@github.com:acme/api.git";

fn name(raw: &str) -> BranchName {
    BranchName::parse(raw).unwrap()
}

fn api() -> RepositoryPath {
    RepositoryPath::new(API)
}

fn web() -> RepositoryPath {
    RepositoryPath::new(WEB)
}

fn login_checkout() -> WorktreePath {
    WorktreePath::new(LOGIN_CHECKOUT)
}

fn other_checkout() -> WorktreePath {
    WorktreePath::new(OTHER_CHECKOUT)
}

fn injected_failure() -> GitError {
    GitError::Io {
        action: "write".into(),
        detail: "disk full".into(),
    }
}

/// The `api` repository on `main`, without a remote.
struct Fixture {
    git: InMemoryGitRepositories,
}

impl Fixture {
    fn new() -> Self {
        let git = InMemoryGitRepositories::new();
        git.add_repository(&api(), &name(MAIN));
        Self { git }
    }

    fn with_remote() -> Self {
        let fixture = Self::new();
        fixture.git.set_remote(&api(), RemoteUrl::new(ORIGIN_URL));
        fixture
    }

    fn open(&self, path: &str) -> Box<dyn GitRepository> {
        self.git.open(&RepositoryPath::new(path)).unwrap()
    }

    /// The main checkout, with `feature/login` created from `main`.
    fn with_feature(&self) -> Box<dyn GitRepository> {
        let repository = self.open(API);
        repository
            .create_branch(&name(FEATURE), &name(MAIN))
            .unwrap();
        repository
    }

    /// The main checkout, with `feature/login` checked out at the login worktree.
    fn with_feature_checked_out(&self) -> Box<dyn GitRepository> {
        let repository = self.with_feature();
        repository
            .add_worktree(&login_checkout(), &name(FEATURE))
            .unwrap();
        repository
    }

    fn branch(&self, raw: &str) -> LocalBranch {
        self.open(API)
            .branches()
            .unwrap()
            .into_iter()
            .find(|branch| branch.name.as_str() == raw)
            .unwrap()
    }
}

#[test]
fn opens_a_repository_at_its_main_checkout_or_a_worktree() {
    let fixture = Fixture::new();
    fixture.with_feature_checked_out();

    assert_eq!(fixture.open(API).current_branch(), Some(name(MAIN)));
    assert_eq!(
        fixture.open(LOGIN_CHECKOUT).current_branch(),
        Some(name(FEATURE))
    );
}

#[test]
fn opening_an_unknown_folder_fails() {
    let fixture = Fixture::new();

    let result = fixture.git.open(&RepositoryPath::new(UNKNOWN));

    assert!(matches!(result, Err(GitError::NotARepository { .. })));
}

#[test]
fn recognizes_only_main_checkouts_as_repository_roots() {
    let fixture = Fixture::new();
    fixture.with_feature_checked_out();

    assert!(fixture.git.is_repository_root(Path::new(API)));
    assert!(!fixture.git.is_repository_root(Path::new(LOGIN_CHECKOUT)));
}

#[test]
fn creates_a_branch_from_its_base_without_commits_beyond_it() {
    let fixture = Fixture::new();

    let repository = fixture.with_feature();

    assert!(repository.has_branch(&name(FEATURE)));
    assert!(!fixture.git.has_commits_beyond_base(&api(), &name(FEATURE)));
}

#[test]
fn a_commit_puts_a_branch_beyond_its_base() {
    let fixture = Fixture::new();
    fixture.with_feature();

    fixture.git.commit(&api(), &name(FEATURE));

    assert!(fixture.git.has_commits_beyond_base(&api(), &name(FEATURE)));
}

#[test]
fn refuses_to_create_a_branch_twice_or_from_a_missing_base() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature();

    assert_eq!(
        repository.create_branch(&name(FEATURE), &name(MAIN)),
        Err(GitError::BranchAlreadyExists {
            name: FEATURE.into()
        })
    );
    assert_eq!(
        repository.create_branch(&name(ALPHA), &name(MISSING)),
        Err(GitError::BranchNotFound {
            name: MISSING.into()
        })
    );
}

#[test]
fn lists_branch_names_sorted() {
    let fixture = Fixture::new();
    let repository = fixture.open(API);
    repository.create_branch(&name(ZETA), &name(MAIN)).unwrap();
    repository.create_branch(&name(ALPHA), &name(MAIN)).unwrap();

    let names = repository.branch_names().unwrap();

    assert_eq!(names, [name(ALPHA), name(MAIN), name(ZETA)]);
}

#[test]
fn lists_the_main_checkout_then_worktrees() {
    let fixture = Fixture::new();

    let checkouts = fixture.with_feature_checked_out().worktrees().unwrap();

    assert_eq!(
        checkouts,
        [
            Checkout {
                path: api(),
                branch: Some(name(MAIN)),
                is_main: true
            },
            Checkout {
                path: RepositoryPath::new(LOGIN_CHECKOUT),
                branch: Some(name(FEATURE)),
                is_main: false
            }
        ]
    );
}

#[test]
fn reports_where_a_branch_is_checked_out() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature_checked_out();

    assert_eq!(
        repository.worktree_for(&name(FEATURE)).unwrap(),
        Some(login_checkout())
    );
    assert_eq!(repository.worktree_for(&name(MISSING)).unwrap(), None);
}

#[test]
fn refuses_a_worktree_for_a_missing_branch() {
    let fixture = Fixture::new();

    let result = fixture
        .open(API)
        .add_worktree(&login_checkout(), &name(MISSING));

    assert_eq!(
        result,
        Err(GitError::BranchNotFound {
            name: MISSING.into()
        })
    );
}

#[test]
fn refuses_a_worktree_for_a_branch_checked_out_in_another_worktree() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature_checked_out();

    let result = repository.add_worktree(&other_checkout(), &name(FEATURE));

    assert_eq!(
        result,
        Err(GitError::BranchCheckedOut {
            name: FEATURE.into(),
            worktree: LOGIN_CHECKOUT.into()
        })
    );
    assert_eq!(repository.worktrees().unwrap().len(), 2);
}

#[test]
fn refuses_a_worktree_for_the_branch_of_the_main_checkout() {
    let fixture = Fixture::new();

    let result = fixture
        .open(API)
        .add_worktree(&login_checkout(), &name(MAIN));

    assert_eq!(
        result,
        Err(GitError::BranchCheckedOut {
            name: MAIN.into(),
            worktree: API.into()
        })
    );
}

#[test]
fn refuses_a_worktree_at_a_path_already_in_use() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature_checked_out();
    repository.create_branch(&name(ALPHA), &name(MAIN)).unwrap();

    let result = repository.add_worktree(&login_checkout(), &name(ALPHA));

    assert!(matches!(result, Err(GitError::CommandFailed { .. })));
}

#[test]
fn checks_out_a_branch_whose_previous_worktree_folder_was_deleted() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature_checked_out();
    fixture
        .git
        .delete_worktree_folder(&api(), &login_checkout());

    let worktree = repository.add_worktree(&other_checkout(), &name(FEATURE));

    assert_eq!(worktree, Ok(other_checkout()));
}

#[test]
fn pruning_frees_a_branch_whose_worktree_folder_was_deleted() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature_checked_out();
    fixture
        .git
        .delete_worktree_folder(&api(), &login_checkout());

    repository.prune_worktrees().unwrap();

    assert_eq!(repository.worktree_for(&name(FEATURE)).unwrap(), None);
}

#[test]
fn removes_a_worktree() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature_checked_out();

    repository.remove_worktree(&login_checkout()).unwrap();

    assert_eq!(repository.worktrees().unwrap().len(), 1);
    assert!(fixture
        .git
        .open(&RepositoryPath::new(LOGIN_CHECKOUT))
        .is_err());
}

#[test]
fn refuses_to_remove_a_folder_that_is_not_a_worktree() {
    let fixture = Fixture::new();

    let result = fixture.open(API).remove_worktree(&login_checkout());

    assert!(matches!(result, Err(GitError::CommandFailed { .. })));
}

#[test]
fn refuses_to_delete_a_checked_out_branch() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature_checked_out();

    let result = repository.delete_branch(&name(FEATURE), true);

    assert_eq!(
        result,
        Err(GitError::BranchCheckedOut {
            name: FEATURE.into(),
            worktree: LOGIN_CHECKOUT.into()
        })
    );
}

#[test]
fn deletes_merged_branches_and_needs_force_for_unmerged_ones() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature();
    repository
        .create_branch(&name(MERGED), &name(MAIN))
        .unwrap();
    fixture.git.commit(&api(), &name(FEATURE));

    repository.delete_branch(&name(MERGED), false).unwrap();
    let unforced = repository.delete_branch(&name(FEATURE), false);
    repository.delete_branch(&name(FEATURE), true).unwrap();

    assert!(matches!(unforced, Err(GitError::CommandFailed { .. })));
    assert_eq!(repository.branch_names().unwrap(), [name(MAIN)]);
}

#[test]
fn a_pushed_branch_counts_as_merged_into_its_upstream() {
    let fixture = Fixture::with_remote();
    let repository = fixture.with_feature_checked_out();
    fixture.git.publish(&api(), &name(FEATURE));
    fixture.git.commit(&api(), &name(FEATURE));
    fixture.open(LOGIN_CHECKOUT).push().unwrap();
    repository.remove_worktree(&login_checkout()).unwrap();

    let result = repository.delete_branch(&name(FEATURE), false);

    assert_eq!(result, Ok(()));
}

#[test]
fn describes_local_branches() {
    let fixture = Fixture::with_remote();
    fixture.git.publish(&api(), &name(MAIN));
    let repository = fixture.with_feature_checked_out();
    repository
        .create_branch(&name(MERGED), &name(MAIN))
        .unwrap();
    fixture.git.commit(&api(), &name(FEATURE));

    let main = fixture.branch(MAIN);
    let feature = fixture.branch(FEATURE);
    let merged = fixture.branch(MERGED);

    assert!(main.is_current && main.is_checked_out && main.has_upstream && main.is_merged);
    assert!(!feature.is_current && feature.is_checked_out);
    assert!(!feature.is_merged && !feature.has_upstream);
    assert!(merged.is_merged && !merged.is_checked_out);
}

#[test]
fn flags_a_branch_whose_upstream_was_deleted() {
    let fixture = Fixture::with_remote();
    fixture.with_feature();
    fixture.git.publish(&api(), &name(FEATURE));

    fixture.git.delete_remote_branch(&api(), &name(FEATURE));

    let feature = fixture.branch(FEATURE);
    assert!(feature.upstream_gone && !feature.has_upstream);
}

#[test]
fn reports_the_origin_remote() {
    let fixture = Fixture::with_remote();

    assert_eq!(
        fixture.open(API).remote_url(),
        Some(RemoteUrl::new(ORIGIN_URL))
    );
}

#[test]
fn network_operations_need_an_origin_remote() {
    let fixture = Fixture::new();
    let repository = fixture.open(API);

    assert_eq!(
        repository.fetch_branch(&name(FEATURE)),
        Err(GitError::NoRemote)
    );
    assert_eq!(repository.pull(), Err(GitError::NoRemote));
    assert_eq!(repository.push(), Err(GitError::NoRemote));
    assert_eq!(repository.remote_url(), None);
}

#[test]
fn fetches_a_remote_branch_into_a_local_one() {
    let fixture = Fixture::with_remote();
    let repository = fixture.with_feature();
    fixture.git.publish(&api(), &name(FEATURE));
    repository.delete_branch(&name(FEATURE), true).unwrap();

    repository.fetch_branch(&name(FEATURE)).unwrap();

    assert!(repository.has_branch(&name(FEATURE)));
}

#[test]
fn fetching_a_branch_missing_on_the_remote_fails() {
    let fixture = Fixture::with_remote();

    let result = fixture.open(API).fetch_branch(&name(MISSING));

    assert!(matches!(result, Err(GitError::CommandFailed { .. })));
}

#[test]
fn push_and_pull_need_an_upstream() {
    let fixture = Fixture::with_remote();
    let repository = fixture.open(API);

    assert_eq!(repository.push(), Err(GitError::NoUpstream));
    assert_eq!(repository.pull(), Err(GitError::NoUpstream));
}

#[test]
fn pull_fast_forwards_to_the_upstream() {
    let fixture = Fixture::with_remote();
    fixture.git.publish(&api(), &name(MAIN));
    fixture.git.commit_on_remote(&api(), &name(MAIN));

    fixture.open(API).pull().unwrap();

    assert!(fixture.git.has_commits_beyond_base(&api(), &name(MAIN)));
}

#[test]
fn push_and_pull_refuse_a_diverged_branch() {
    let fixture = Fixture::with_remote();
    fixture.git.publish(&api(), &name(MAIN));
    fixture.git.commit_on_remote(&api(), &name(MAIN));
    fixture.git.commit(&api(), &name(MAIN));
    let repository = fixture.open(API);

    assert!(matches!(
        repository.pull(),
        Err(GitError::NotFastForward { .. })
    ));
    assert!(matches!(
        repository.push(),
        Err(GitError::NotFastForward { .. })
    ));
}

#[test]
fn diffs_are_empty_and_need_an_existing_base() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature();

    assert_eq!(
        repository.diff_stats_against(Some(&name(FEATURE))),
        Ok(DiffStats::default())
    );
    assert_eq!(repository.diff_against(&name(FEATURE)), Ok(String::new()));
    assert_eq!(
        repository.diff_against(&name(MISSING)),
        Err(GitError::BranchNotFound {
            name: MISSING.into()
        })
    );
}

#[test]
fn a_failing_operation_returns_the_error_and_changes_nothing() {
    let fixture = Fixture::new();
    let repository = fixture.with_feature();
    fixture
        .git
        .fail_on(Operation::AddWorktree, injected_failure());

    let result = repository.add_worktree(&login_checkout(), &name(FEATURE));

    assert_eq!(result, Err(injected_failure()));
    assert_eq!(repository.worktree_for(&name(FEATURE)).unwrap(), None);
}

#[test]
fn a_failure_can_be_limited_to_one_repository() {
    let fixture = Fixture::new();
    fixture.git.add_repository(&web(), &name(MAIN));
    fixture
        .git
        .fail_on_in(&web(), Operation::CreateBranch, injected_failure());

    let in_api = fixture.open(API).create_branch(&name(FEATURE), &name(MAIN));
    let in_web = fixture.open(WEB).create_branch(&name(FEATURE), &name(MAIN));

    assert_eq!(in_api, Ok(()));
    assert_eq!(in_web, Err(injected_failure()));
}

#[test]
fn opening_can_be_told_to_fail() {
    let fixture = Fixture::new();
    fixture.git.fail_on(Operation::Open, injected_failure());

    let result = fixture.git.open(&api());

    assert_eq!(result.err(), Some(injected_failure()));
}
