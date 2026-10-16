//! Context checkout steps on disk: one worktree or symlink per repository under
//! `.worktrees/<context>/<repository>`, the post-checkout command, and the
//! project's configured symlinks.
//!
//! TEMPORARY: superseded by the `workspace` service's create and delete use
//! cases, which also undo them on failure; removed once the frontend calls
//! those (#43). The symlink and post-checkout helpers already live in
//! `workspace::infra::file_system_checkout_files`.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::git::commands::GitCommandError;
use crate::git::domain::branch_name::BranchName;
use crate::git::domain::error::GitError;
use crate::git::domain::paths::{RepositoryPath, WorktreePath};
use crate::git::domain::ports::{GitRepositories, GitRepository};
use crate::git::{run_blocking, Git};
use crate::workspace::infra::file_system_checkout_files::{create_symlink, run_post_checkout};

const WORKTREES_FOLDER: &str = ".worktrees";
const DEFAULT_CONTEXT_FOLDER: &str = "default";

/// One repository of a context, as sent by the frontend. Field names are the wire format.
#[derive(Deserialize, specta::Type)]
pub struct ContextRepositoryInput {
    pub rel_path: String,
    pub name: String,
    pub branch: String,
    pub base_branch: String,
    #[serde(default)]
    pub linked: bool,
    #[serde(default)]
    pub post_checkout_command: Option<String>,
}

#[derive(Serialize, specta::Type)]
pub struct ContextRepositoryResult {
    pub rel_path: String,
    pub worktree_path: String,
    pub branch: String,
}

impl ContextRepositoryInput {
    fn open(
        &self,
        repositories: &dyn GitRepositories,
        project_folder: &Path,
    ) -> Result<Box<dyn GitRepository>, GitCommandError> {
        repositories
            .open(&RepositoryPath::new(project_folder.join(&self.rel_path)))
            .map_err(|error| self.failure(error))
    }

    fn failure(&self, error: GitError) -> GitCommandError {
        let message = format!("{error} (in '{}')", self.rel_path);
        GitCommandError::from(error).with_message(message)
    }
}

/// The `.worktrees/<context>` folder of a project and what lives in it.
struct ContextFolder {
    project_folder: PathBuf,
    path: PathBuf,
}

impl ContextFolder {
    fn new(project_folder: &Path, context_name: &str) -> Self {
        Self {
            project_folder: project_folder.to_path_buf(),
            path: project_folder.join(WORKTREES_FOLDER).join(context_name),
        }
    }

    fn repository_checkout(&self, repository_name: &str) -> PathBuf {
        self.path.join(repository_name)
    }

    /// Where a configured symlink (a path relative to the project) is linked in
    /// the context: under its file name.
    fn configured_link(&self, relative: &str) -> PathBuf {
        let link_name = Path::new(relative)
            .file_name()
            .unwrap_or_else(|| OsStr::new(relative));
        self.path.join(link_name)
    }

    fn remove_if_empty(&self) {
        let is_empty = self
            .path
            .read_dir()
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if is_empty {
            let _ = std::fs::remove_dir(&self.path);
        }
    }
}

/// Removes whatever is at `path`; a symlink is removed as a link, not followed.
fn remove_existing_path(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// A linked repository reuses the base context's checkout through a symlink.
fn link_repository(
    context_folder: &ContextFolder,
    repository_input: &ContextRepositoryInput,
    base_context_name: Option<&str>,
) -> Result<PathBuf, GitCommandError> {
    let checkout = context_folder.repository_checkout(&repository_input.name);
    let target = match base_context_name {
        Some(base) if base != DEFAULT_CONTEXT_FOLDER => {
            ContextFolder::new(&context_folder.project_folder, base)
                .repository_checkout(&repository_input.name)
        }
        _ => context_folder
            .project_folder
            .join(&repository_input.rel_path),
    };
    if let Some(parent) = checkout.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            GitCommandError::io(format!("Failed to create parent directory: {error}"))
        })?;
    }
    // Clear a leftover entry so re-creating a context is idempotent.
    remove_existing_path(&checkout).map_err(|error| {
        GitCommandError::io(format!(
            "Failed to clear existing path for '{}': {error}",
            repository_input.name
        ))
    })?;
    create_symlink(&target, &checkout).map_err(|error| {
        GitCommandError::io(format!(
            "Failed to create symlink for '{}': {error}",
            repository_input.name
        ))
    })?;
    Ok(checkout)
}

/// Checks the repository's branch out as a worktree, creating the branch from
/// its base first when it does not exist yet.
fn check_out_repository(
    repositories: &dyn GitRepositories,
    context_folder: &ContextFolder,
    repository_input: &ContextRepositoryInput,
) -> Result<PathBuf, GitCommandError> {
    let repository = repository_input.open(repositories, &context_folder.project_folder)?;
    let failure = |error| repository_input.failure(error);
    let branch = BranchName::parse(&repository_input.branch).map_err(failure)?;
    if !repository.has_branch(&branch) {
        let base = BranchName::parse(&repository_input.base_branch).map_err(failure)?;
        match repository.create_branch(&branch, &base) {
            // Another context created it in the meantime; check that one out.
            Ok(()) | Err(GitError::BranchAlreadyExists { .. }) => {}
            Err(error) => return Err(failure(error)),
        }
    }
    let checkout = WorktreePath::new(context_folder.repository_checkout(&repository_input.name));
    let worktree = repository
        .add_worktree(&checkout, &branch)
        .map_err(failure)?;
    Ok(worktree.as_path().to_path_buf())
}

fn link_configured_paths(
    context_folder: &ContextFolder,
    symlinks: &[String],
) -> Result<(), GitCommandError> {
    for relative in symlinks {
        let link = context_folder.configured_link(relative);
        if link.exists() || link.is_symlink() {
            continue;
        }
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                GitCommandError::io(format!(
                    "Failed to create directory for symlink '{relative}': {error}"
                ))
            })?;
        }
        create_symlink(&context_folder.project_folder.join(relative), &link).map_err(|error| {
            GitCommandError::io(format!("Failed to symlink '{relative}': {error}"))
        })?;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn create_context(
    git: State<'_, Git>,
    project_path: String,
    context_name: String,
    repos: Vec<ContextRepositoryInput>,
    symlinks: Vec<String>,
    base_context_name: Option<String>,
) -> Result<Vec<ContextRepositoryResult>, GitCommandError> {
    let repositories = git.repositories();
    run_blocking(move || {
        let context_folder = ContextFolder::new(Path::new(&project_path), &context_name);
        let mut checkouts = Vec::with_capacity(repos.len());

        for repository_input in &repos {
            let checkout = if repository_input.linked {
                link_repository(
                    &context_folder,
                    repository_input,
                    base_context_name.as_deref(),
                )?
            } else {
                check_out_repository(repositories.as_ref(), &context_folder, repository_input)?
            };
            checkouts.push(checkout);
        }

        // Only once every repository is checked out, so a failure above runs no command.
        for (repository_input, checkout) in repos.iter().zip(&checkouts) {
            if !repository_input.linked {
                if let Some(command) = &repository_input.post_checkout_command {
                    run_post_checkout(command, checkout);
                }
            }
        }

        link_configured_paths(&context_folder, &symlinks)?;

        Ok(repos
            .iter()
            .zip(checkouts)
            .map(|(repository_input, checkout)| ContextRepositoryResult {
                rel_path: repository_input.rel_path.clone(),
                worktree_path: WorktreePath::new(checkout).display(),
                branch: repository_input.branch.clone(),
            })
            .collect())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_context(
    git: State<'_, Git>,
    project_path: String,
    context_name: String,
    repos: Vec<ContextRepositoryInput>,
    symlinks: Vec<String>,
) -> Result<(), GitCommandError> {
    let repositories = git.repositories();
    run_blocking(move || {
        let context_folder = ContextFolder::new(Path::new(&project_path), &context_name);

        for repository_input in &repos {
            let checkout = context_folder.repository_checkout(&repository_input.name);
            if checkout.is_symlink() {
                std::fs::remove_file(&checkout).map_err(|error| {
                    GitCommandError::io(format!(
                        "Failed to remove symlink '{}': {error}",
                        checkout.display()
                    ))
                })?;
            } else if checkout.exists() {
                repository_input
                    .open(repositories.as_ref(), &context_folder.project_folder)?
                    .remove_worktree(&WorktreePath::new(&checkout))
                    .map_err(|error| repository_input.failure(error))?;
            }
        }

        for relative in &symlinks {
            let link = context_folder.configured_link(relative);
            if link.is_symlink() {
                let _ = std::fs::remove_file(&link);
            }
        }

        context_folder.remove_if_empty();
        Ok(())
    })
    .await
}
