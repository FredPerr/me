//! Reads a repository's pull request template. It runs in the backend so it is
//! not limited by the frontend file-system plugin's path scope (worktrees live
//! in arbitrary project folders).
use std::path::{Path, PathBuf};

use crate::git::commands::GitCommandError;
use crate::git::domain::error::GitError;

fn read_failure(error: std::io::Error) -> GitCommandError {
    GitError::io("read the pull request template", error).into()
}

/// Single-file pull request template locations GitHub recognizes, relative to
/// the repository root (the root, `.github/`, or `docs/`; name is case-insensitive on GitHub).
const TEMPLATE_FILE_CANDIDATES: &[&str] = &[
    ".github/PULL_REQUEST_TEMPLATE.md",
    ".github/pull_request_template.md",
    "PULL_REQUEST_TEMPLATE.md",
    "pull_request_template.md",
    "docs/PULL_REQUEST_TEMPLATE.md",
    "docs/pull_request_template.md",
];

/// Directories that may hold multiple named templates. When several exist the
/// first alphabetically is used as a reasonable default.
const TEMPLATE_FOLDER_CANDIDATES: &[&str] = &[
    ".github/PULL_REQUEST_TEMPLATE",
    "PULL_REQUEST_TEMPLATE",
    "docs/PULL_REQUEST_TEMPLATE",
];

/// The pull request template of the worktree at `path`, or `None` when it has none.
#[tauri::command]
#[specta::specta]
pub async fn read_pull_request_template(path: String) -> Result<Option<String>, GitCommandError> {
    let root = Path::new(&path);

    for candidate in TEMPLATE_FILE_CANDIDATES {
        let file = root.join(candidate);
        if file.is_file() {
            return std::fs::read_to_string(&file)
                .map(Some)
                .map_err(read_failure);
        }
    }

    for candidate in TEMPLATE_FOLDER_CANDIDATES {
        let dir = root.join(candidate);
        if !dir.is_dir() {
            continue;
        }
        let mut markdown_files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map_err(read_failure)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            })
            .collect();
        markdown_files.sort();
        if let Some(first) = markdown_files.first() {
            return std::fs::read_to_string(first)
                .map(Some)
                .map_err(read_failure);
        }
    }

    Ok(None)
}
