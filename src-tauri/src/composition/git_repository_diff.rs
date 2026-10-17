//! `ai_assist`'s `RepositoryDiff`, read from `git` and the pull-request-template
//! reader. The only place `ai_assist`'s diff port meets `git`: `ai_assist`
//! depends on the port, `git` owns repositories, and this adapter reads the
//! same diff and template the frontend read through `getWorktreeDiff` and
//! `readPullRequestTemplate` — so the prompt sees exactly what it did before.
//!
//! Both reads are blocking disk work, so they run off the async runtime.

use std::sync::Arc;

use async_trait::async_trait;

use crate::ai_assist::domain::ports::{
    RepositoryDiff, RepositoryDiffContents, RepositoryDiffUnavailable,
};
use crate::git::domain::branch_name::BranchName;
use crate::git::domain::paths::RepositoryPath;
use crate::git::domain::ports::GitRepositories;
use crate::pull_request_template::pull_request_template_at;

pub struct GitRepositoryDiff {
    repositories: Arc<dyn GitRepositories>,
}

impl GitRepositoryDiff {
    pub fn new(repositories: Arc<dyn GitRepositories>) -> Self {
        Self { repositories }
    }
}

#[async_trait]
impl RepositoryDiff for GitRepositoryDiff {
    async fn read(
        &self,
        worktree_path: &str,
        base_branch: &str,
    ) -> Result<RepositoryDiffContents, RepositoryDiffUnavailable> {
        let repositories = Arc::clone(&self.repositories);
        let worktree_path = worktree_path.to_string();
        let base_branch = base_branch.to_string();

        tauri::async_runtime::spawn_blocking(move || {
            read_diff_and_template(repositories.as_ref(), &worktree_path, &base_branch)
        })
        .await
        .map_err(|error| RepositoryDiffUnavailable {
            reason: format!("reading the diff did not finish: {error}"),
        })?
    }
}

/// Opens the worktree, reads its diff against `base_branch`, and reads the pull
/// request template next to it. Any git or parse failure is a single
/// [`RepositoryDiffUnavailable`] with a readable reason.
fn read_diff_and_template(
    repositories: &dyn GitRepositories,
    worktree_path: &str,
    base_branch: &str,
) -> Result<RepositoryDiffContents, RepositoryDiffUnavailable> {
    let base = BranchName::parse(base_branch).map_err(unavailable)?;
    let repository = repositories
        .open(&RepositoryPath::new(worktree_path))
        .map_err(unavailable)?;
    let diff = repository.diff_against(&base).map_err(unavailable)?;
    let template =
        pull_request_template_at(worktree_path).map_err(|error| RepositoryDiffUnavailable {
            reason: error.message,
        })?;
    Ok(RepositoryDiffContents { diff, template })
}

fn unavailable(error: impl std::fmt::Display) -> RepositoryDiffUnavailable {
    RepositoryDiffUnavailable {
        reason: error.to_string(),
    }
}
