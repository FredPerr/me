use serde::{Deserialize, Serialize};

use crate::git::domain::branch_name::BranchName;

use super::identifiers::RepositoryId;

/// The branch a context uses in one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextBranch {
    pub repository: RepositoryId,
    pub branch: BranchName,
    pub pull_request: Option<PullRequestLink>,
    pub checkout: ContextCheckout,
}

impl ContextBranch {
    /// A branch checked out in the context's own worktree.
    pub fn new(repository: RepositoryId, branch: BranchName) -> Self {
        Self {
            repository,
            branch,
            pull_request: None,
            checkout: ContextCheckout::Worktree,
        }
    }

    pub fn is_linked(&self) -> bool {
        self.checkout == ContextCheckout::Linked
    }
}

/// How the repository is checked out in the context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextCheckout {
    /// The context's own worktree, on its own branch.
    Worktree,
    // TEMPORARY(legacy linked repositories): remove when no saved context uses them.
    /// A symlink to the base context's checkout. Read and deleted only; no new
    /// context gets one.
    Linked,
}

/// The pull request of a context branch. Holds the saved draft for now;
/// matched pull request data comes with git hosting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestLink {
    pub draft: PullRequestDraft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestDraft {
    pub title: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_branch() -> ContextBranch {
        ContextBranch::new(
            RepositoryId::parse("repository-1").unwrap(),
            BranchName::parse("feature/x").unwrap(),
        )
    }

    #[test]
    fn a_new_branch_has_its_own_worktree_and_no_pull_request() {
        let branch = build_branch();
        assert!(!branch.is_linked());
        assert_eq!(branch.pull_request, None);
    }

    #[test]
    fn serializes_absent_values_as_null() {
        let json = serde_json::to_value(build_branch()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "repository": "repository-1",
                "branch": "feature/x",
                "pullRequest": null,
                "checkout": "worktree",
            })
        );
    }
}
