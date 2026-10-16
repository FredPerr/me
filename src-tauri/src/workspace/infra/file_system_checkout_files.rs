use std::path::Path;
use std::process::Command;

use crate::workspace::domain::ports::{CheckoutFiles, CheckoutFilesError};

/// The checkout steps on the real file system, with `sh` for post-checkout commands.
pub struct FileSystemCheckoutFiles;

impl CheckoutFiles for FileSystemCheckoutFiles {
    fn create_symlink(&self, target: &Path, link: &Path) -> Result<bool, CheckoutFilesError> {
        if link.exists() || link.is_symlink() {
            return Ok(false);
        }
        let failure = |error: std::io::Error| CheckoutFilesError {
            path: link.display().to_string(),
            reason: error.to_string(),
        };
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent).map_err(failure)?;
        }
        create_symlink(target, link).map_err(failure)?;
        Ok(true)
    }

    fn remove_symlink(&self, link: &Path) -> Result<(), CheckoutFilesError> {
        if !link.is_symlink() {
            return Ok(());
        }
        std::fs::remove_file(link).map_err(|error| CheckoutFilesError {
            path: link.display().to_string(),
            reason: error.to_string(),
        })
    }

    fn remove_folder_if_empty(&self, folder: &Path) {
        let is_empty = folder
            .read_dir()
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if is_empty {
            let _ = std::fs::remove_dir(folder);
        }
    }

    fn run_post_checkout(&self, command: &str, checkout: &Path) {
        run_post_checkout(command, checkout);
    }
}

pub fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            std::os::windows::fs::symlink_file(target, link)
        }
    }
}

/// Runs `command` with `sh` in `checkout`, ignoring how it ends.
pub fn run_post_checkout(command: &str, checkout: &Path) {
    if command.trim().is_empty() {
        return;
    }
    let _ = Command::new("sh")
        .args(["-c", command])
        .current_dir(checkout)
        .output();
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TARGET_FILE: &str = ".env";
    const LINK_PATH: &str = "context/nested/.env";
    const CONTEXT_FOLDER: &str = "context";
    const MARKER_FILE: &str = "installed";

    struct Fixture {
        temp: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let fixture = Self {
                temp: TempDir::new().unwrap(),
            };
            std::fs::write(fixture.path(TARGET_FILE), "SECRET=1\n").unwrap();
            fixture
        }

        fn path(&self, relative: &str) -> std::path::PathBuf {
            self.temp.path().join(relative)
        }

        fn link(&self) -> Result<bool, CheckoutFilesError> {
            FileSystemCheckoutFiles.create_symlink(&self.path(TARGET_FILE), &self.path(LINK_PATH))
        }
    }

    #[test]
    fn creates_a_symlink_and_its_parent_folders() {
        let fixture = Fixture::new();

        assert_eq!(fixture.link(), Ok(true));
        assert_eq!(
            std::fs::read_link(fixture.path(LINK_PATH)).unwrap(),
            fixture.path(TARGET_FILE)
        );
    }

    #[test]
    fn leaves_an_existing_entry_in_place() {
        let fixture = Fixture::new();
        fixture.link().unwrap();

        assert_eq!(fixture.link(), Ok(false));
    }

    #[test]
    fn removes_a_symlink_but_not_its_target_then_the_empty_folders() {
        let fixture = Fixture::new();
        fixture.link().unwrap();

        FileSystemCheckoutFiles
            .remove_symlink(&fixture.path(LINK_PATH))
            .unwrap();
        FileSystemCheckoutFiles.remove_folder_if_empty(&fixture.path("context/nested"));
        FileSystemCheckoutFiles.remove_folder_if_empty(&fixture.path(CONTEXT_FOLDER));

        assert!(fixture.path(TARGET_FILE).is_file());
        assert!(!fixture.path(CONTEXT_FOLDER).exists());
    }

    #[test]
    fn keeps_a_folder_that_is_not_empty() {
        let fixture = Fixture::new();
        fixture.link().unwrap();

        FileSystemCheckoutFiles.remove_folder_if_empty(&fixture.path(CONTEXT_FOLDER));

        assert!(fixture.path(LINK_PATH).is_symlink());
    }

    #[test]
    fn runs_the_post_checkout_command_in_the_checkout() {
        let fixture = Fixture::new();

        FileSystemCheckoutFiles
            .run_post_checkout(&format!("touch {MARKER_FILE}"), fixture.temp.path());

        assert!(fixture.path(MARKER_FILE).is_file());
    }
}
