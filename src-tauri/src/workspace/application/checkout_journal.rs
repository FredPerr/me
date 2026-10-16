use std::path::{Path, PathBuf};

use crate::git::domain::branch_name::BranchName;
use crate::git::domain::paths::WorktreePath;
use crate::git::domain::ports::GitRepository;
use crate::workspace::domain::ports::CheckoutFiles;

/// One step of a context checkout that can be undone.
enum CheckoutStep<'a> {
    CreatedBranch {
        repository: &'a dyn GitRepository,
        branch: &'a BranchName,
    },
    AddedWorktree {
        repository: &'a dyn GitRepository,
        worktree: &'a WorktreePath,
    },
    CreatedSymlink(PathBuf),
}

/// The steps a context checkout has done so far, so a failure can undo them
/// in reverse order.
#[derive(Default)]
pub struct CheckoutJournal<'a> {
    steps: Vec<CheckoutStep<'a>>,
}

impl<'a> CheckoutJournal<'a> {
    pub fn created_branch(&mut self, repository: &'a dyn GitRepository, branch: &'a BranchName) {
        self.steps
            .push(CheckoutStep::CreatedBranch { repository, branch });
    }

    pub fn added_worktree(
        &mut self,
        repository: &'a dyn GitRepository,
        worktree: &'a WorktreePath,
    ) {
        self.steps.push(CheckoutStep::AddedWorktree {
            repository,
            worktree,
        });
    }

    pub fn created_symlink(&mut self, link: PathBuf) {
        self.steps.push(CheckoutStep::CreatedSymlink(link));
    }

    /// Best effort: a step that can't be undone doesn't stop the others, and
    /// the original failure is what gets reported.
    pub fn undo(self, files: &dyn CheckoutFiles, context_folder: &Path) {
        for step in self.steps.into_iter().rev() {
            let outcome = match step {
                CheckoutStep::CreatedSymlink(link) => files
                    .remove_symlink(&link)
                    .map_err(|error| error.to_string()),
                CheckoutStep::AddedWorktree {
                    repository,
                    worktree,
                } => repository
                    .remove_worktree(worktree)
                    .map_err(|error| error.to_string()),
                CheckoutStep::CreatedBranch { repository, branch } => repository
                    .delete_branch(branch, true)
                    .map_err(|error| error.to_string()),
            };
            if let Err(reason) = outcome {
                eprintln!("[workspace] undoing a context checkout step failed: {reason}");
            }
        }
        files.remove_folder_if_empty(context_folder);
    }
}
