//! `ai_assist`'s `TempWorkspace`, backed by `tempfile`. A context suggestion
//! runs in a fresh empty directory so it can touch no worktree; this adapter
//! creates that directory and, through the handle's `Drop`, removes it when the
//! run ends.
//!
//! `tempfile::TempDir` is the whole implementation: it makes a uniquely named
//! directory under the OS temp dir and deletes it (and anything the run wrote)
//! when dropped. The `create` call is marked blocking off the async runtime
//! because it touches the file system.

use async_trait::async_trait;
use tempfile::TempDir;

use crate::ai_assist::domain::ports::{TempDirectory, TempWorkspace, TempWorkspaceUnavailable};

pub struct TempfileTempWorkspace;

#[async_trait]
impl TempWorkspace for TempfileTempWorkspace {
    async fn create(&self) -> Result<Box<dyn TempDirectory>, TempWorkspaceUnavailable> {
        let created = tauri::async_runtime::spawn_blocking(TempDir::new)
            .await
            .map_err(|error| TempWorkspaceUnavailable {
                reason: format!("The temporary folder task failed: {error}"),
            })?
            .map_err(|error| TempWorkspaceUnavailable {
                reason: format!("A temporary folder could not be created: {error}"),
            })?;
        Ok(Box::new(TempfileTempDirectory { inner: created }))
    }
}

/// Holds the `TempDir` alive for the run; dropping it removes the directory.
struct TempfileTempDirectory {
    inner: TempDir,
}

impl TempDirectory for TempfileTempDirectory {
    fn path(&self) -> String {
        self.inner.path().to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn creates_a_fresh_directory_and_removes_it_on_drop() {
        let workspace = TempfileTempWorkspace;

        let directory = workspace.create().await.unwrap();
        let path = PathBuf::from(directory.path());
        assert!(path.is_dir(), "the directory exists while the handle lives");

        drop(directory);
        assert!(
            !path.exists(),
            "the directory is removed when the handle drops"
        );
    }
}
