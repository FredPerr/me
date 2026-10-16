use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::git_hosting::domain::hosted_pull_request::HostedPullRequest;
use crate::git_hosting::domain::hosted_repository::HostedRepository;
use crate::git_hosting::domain::ports::Clock;

/// How long a branch's pull request is reused before the Git host is asked again.
pub const BRANCH_LOOKUP_LIFETIME: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BranchKey {
    repository: HostedRepository,
    branch: String,
}

struct BranchLookup {
    looked_up_at: Instant,
    pull_request: Option<HostedPullRequest>,
}

/// Branch pull requests the Git hosts answered recently, so redrawing a board
/// does not ask them again. Only answers are kept, never failures.
pub struct BranchPullRequestCache {
    clock: Arc<dyn Clock>,
    lookups: Mutex<HashMap<BranchKey, BranchLookup>>,
}

impl BranchPullRequestCache {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            lookups: Mutex::new(HashMap::new()),
        }
    }

    /// `Some` holds the answer of a lookup younger than [`BRANCH_LOOKUP_LIFETIME`].
    pub fn find(
        &self,
        repository: &HostedRepository,
        branch: &str,
    ) -> Option<Option<HostedPullRequest>> {
        let key = key(repository, branch);
        let now = self.clock.now();
        let mut lookups = self.lookups();
        match lookups.get(&key) {
            Some(lookup) if now.duration_since(lookup.looked_up_at) < BRANCH_LOOKUP_LIFETIME => {
                Some(lookup.pull_request.clone())
            }
            Some(_) => {
                lookups.remove(&key);
                None
            }
            None => None,
        }
    }

    pub fn remember(
        &self,
        repository: &HostedRepository,
        branch: &str,
        pull_request: Option<HostedPullRequest>,
    ) {
        let lookup = BranchLookup {
            looked_up_at: self.clock.now(),
            pull_request,
        };
        self.lookups().insert(key(repository, branch), lookup);
    }

    pub fn forget(&self, repository: &HostedRepository, branch: &str) {
        self.lookups().remove(&key(repository, branch));
    }

    fn lookups(&self) -> std::sync::MutexGuard<'_, HashMap<BranchKey, BranchLookup>> {
        // A panic while holding the lock leaves the map consistent: every
        // change is a single insert or remove.
        self.lookups
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn key(repository: &HostedRepository, branch: &str) -> BranchKey {
    BranchKey {
        repository: repository.clone(),
        branch: branch.to_string(),
    }
}
