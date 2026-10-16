use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// One lock per repository, keyed by the git folder its worktrees share.
///
/// Worktrees of one repository share refs, the worktree list and `FETCH_HEAD`;
/// two concurrent writes (for example two fetches) corrupt that shared state.
pub fn for_common_dir(common_dir: &Path) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let key = std::fs::canonicalize(common_dir).unwrap_or_else(|_| common_dir.to_path_buf());
    let mut locks = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    locks.entry(key).or_default().clone()
}
