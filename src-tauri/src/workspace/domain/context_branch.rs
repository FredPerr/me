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

    /// A branch checked out from a matched pull request, recording the link.
    pub fn from_pull_request(
        repository: RepositoryId,
        branch: BranchName,
        matched: MatchedPullRequest,
    ) -> Self {
        Self {
            pull_request: Some(PullRequestLink::matched(matched)),
            ..Self::new(repository, branch)
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

/// The pull request of a context branch: the draft the user is composing and,
/// when the branch was created from a pull request, the match it was built
/// from. Both are optional and default to absent, so a context branch saved
/// before either existed still loads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestLink {
    #[serde(default)]
    pub draft: Option<PullRequestDraft>,
    #[serde(default)]
    pub matched: Option<MatchedPullRequest>,
}

impl PullRequestLink {
    /// The link of a branch created from a matched pull request, no draft yet.
    pub fn matched(matched: MatchedPullRequest) -> Self {
        Self {
            draft: None,
            matched: Some(matched),
        }
    }

    /// Replaces the draft, keeping any recorded match.
    pub fn with_draft(self, draft: PullRequestDraft) -> Self {
        Self {
            draft: Some(draft),
            ..self
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestDraft {
    pub title: String,
    pub description: String,
}

/// The pull request a context branch was created from. `host` is the Git host
/// kind as a plain string ("github", "bitbucket"): the workspace context does
/// not depend on `git_hosting`, so it keeps no typed host enum of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchedPullRequest {
    pub host: String,
    pub number: u32,
    pub url: String,
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

    const HOST: &str = "github";
    const NUMBER: u32 = 12;
    const URL: &str = "https://github.com/acme/api/pull/12";

    fn matched() -> MatchedPullRequest {
        MatchedPullRequest {
            host: HOST.to_string(),
            number: NUMBER,
            url: URL.to_string(),
        }
    }

    #[test]
    fn a_new_branch_has_its_own_worktree_and_no_pull_request() {
        let branch = build_branch();
        assert!(!branch.is_linked());
        assert_eq!(branch.pull_request, None);
    }

    #[test]
    fn a_branch_from_a_pull_request_records_the_match_without_a_draft() {
        let branch = ContextBranch::from_pull_request(
            RepositoryId::parse("repository-1").unwrap(),
            BranchName::parse("feature/x").unwrap(),
            matched(),
        );
        let link = branch.pull_request.unwrap();
        assert_eq!(link.matched, Some(matched()));
        assert_eq!(link.draft, None);
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

    #[test]
    fn loads_a_link_saved_with_only_a_draft() {
        let json = serde_json::json!({
            "draft": { "title": "Add login", "description": "Adds the form" },
        });

        let link: PullRequestLink = serde_json::from_value(json).unwrap();

        assert_eq!(
            link.draft,
            Some(PullRequestDraft {
                title: "Add login".to_string(),
                description: "Adds the form".to_string(),
            })
        );
        assert_eq!(link.matched, None);
    }
}
