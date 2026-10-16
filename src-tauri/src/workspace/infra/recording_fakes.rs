//! Recording fakes of the checkout files and project events ports for the
//! service tests. Files are modelled as a set of symlinks; any symlink can be
//! told to fail.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::workspace::domain::ports::{CheckoutFiles, CheckoutFilesError, ProjectEvents};
use crate::workspace::domain::project_tag::ProjectTag;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A post-checkout command run in a checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostCheckoutRun {
    pub command: String,
    pub checkout: PathBuf,
}

#[derive(Default)]
struct FilesState {
    symlinks: BTreeSet<PathBuf>,
    removed_folders: Vec<PathBuf>,
    post_checkout_runs: Vec<PostCheckoutRun>,
    failing_links: Vec<PathBuf>,
}

#[derive(Default)]
pub struct RecordingCheckoutFiles {
    state: Mutex<FilesState>,
}

impl RecordingCheckoutFiles {
    pub fn symlinks(&self) -> Vec<PathBuf> {
        lock(&self.state).symlinks.iter().cloned().collect()
    }

    pub fn post_checkout_runs(&self) -> Vec<PostCheckoutRun> {
        lock(&self.state).post_checkout_runs.clone()
    }

    pub fn removed_folders(&self) -> Vec<PathBuf> {
        lock(&self.state).removed_folders.clone()
    }

    /// A symlink already on disk, such as a legacy linked repository.
    pub fn add_symlink(&self, link: &str) {
        lock(&self.state).symlinks.insert(PathBuf::from(link));
    }

    /// Creating a symlink at `link` fails from now on.
    pub fn fail_on_link(&self, link: &str) {
        lock(&self.state).failing_links.push(PathBuf::from(link));
    }
}

impl CheckoutFiles for RecordingCheckoutFiles {
    fn create_symlink(&self, _target: &Path, link: &Path) -> Result<bool, CheckoutFilesError> {
        let mut state = lock(&self.state);
        if state.failing_links.iter().any(|failing| failing == link) {
            return Err(CheckoutFilesError {
                path: link.display().to_string(),
                reason: "permission denied".to_string(),
            });
        }
        Ok(state.symlinks.insert(link.to_path_buf()))
    }

    fn remove_symlink(&self, link: &Path) -> Result<(), CheckoutFilesError> {
        lock(&self.state).symlinks.remove(link);
        Ok(())
    }

    fn remove_folder_if_empty(&self, folder: &Path) {
        lock(&self.state).removed_folders.push(folder.to_path_buf());
    }

    fn run_post_checkout(&self, command: &str, checkout: &Path) {
        lock(&self.state).post_checkout_runs.push(PostCheckoutRun {
            command: command.to_string(),
            checkout: checkout.to_path_buf(),
        });
    }
}

#[derive(Default)]
pub struct RecordingProjectEvents {
    changed: Mutex<Vec<ProjectTag>>,
}

impl RecordingProjectEvents {
    pub fn changed_projects(&self) -> Vec<ProjectTag> {
        lock(&self.changed).clone()
    }
}

impl ProjectEvents for RecordingProjectEvents {
    fn project_changed(&self, tag: &ProjectTag) {
        lock(&self.changed).push(tag.clone());
    }
}
