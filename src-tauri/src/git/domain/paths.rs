use std::path::{Path, PathBuf};

use serde::{Serialize, Serializer};

fn display_string(path: &Path) -> String {
    let raw = path.to_string_lossy();
    match raw.strip_suffix('/') {
        Some(stripped) if !stripped.is_empty() => stripped.to_string(),
        _ => raw.into_owned(),
    }
}

macro_rules! path_value {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(PathBuf);

        impl $name {
            pub fn new(path: impl Into<PathBuf>) -> Self {
                Self(path.into())
            }

            pub fn as_path(&self) -> &Path {
                &self.0
            }

            /// The path as text without a trailing slash.
            pub fn display(&self) -> String {
                display_string(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.display())
            }
        }
    };
}

path_value!(
    /// The folder of a repository's main checkout or one of its worktrees.
    RepositoryPath
);

path_value!(
    /// The folder of a linked worktree.
    WorktreePath
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_drops_a_trailing_slash() {
        assert_eq!(
            WorktreePath::new("/work/.worktrees/a/").display(),
            "/work/.worktrees/a"
        );
        assert_eq!(RepositoryPath::new("/work/api").display(), "/work/api");
        assert_eq!(RepositoryPath::new("/").display(), "/");
    }
}
