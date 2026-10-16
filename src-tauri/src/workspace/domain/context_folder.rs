use std::fmt;

use serde::{Deserialize, Serialize};

use super::context_name::ContextName;
use super::error::WorkspaceError;
use super::slug::slugify;

/// The context's folder under `<project>/.worktrees/`. Unique within a
/// project, ignoring case (macOS folders ignore case by default).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContextFolder(String);

impl ContextFolder {
    /// The slug of the name, as every new context gets.
    pub fn from_name(name: &ContextName) -> Result<Self, WorkspaceError> {
        let slug = slugify(name.as_str());
        if slug.is_empty() {
            return Err(WorkspaceError::EmptyContextFolder {
                name: name.to_string(),
            });
        }
        Ok(Self(slug))
    }

    // TEMPORARY(legacy folder names): remove when no saved context lacks a stored folder name.
    /// Contexts saved before folders were slugged live in a folder named after
    /// their raw name.
    pub fn legacy(name: &ContextName) -> Self {
        Self(name.as_str().to_string())
    }

    // TEMPORARY(legacy import): remove with `infra::legacy_project_import`.
    /// A folder name read from an old project file, kept as it was stored.
    pub fn from_stored(folder_name: &str) -> Result<Self, WorkspaceError> {
        let trimmed = folder_name.trim();
        if trimmed.is_empty() {
            return Err(WorkspaceError::EmptyContextFolder {
                name: folder_name.to_string(),
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_same_as(&self, other: &ContextFolder) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

impl fmt::Display for ContextFolder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCENTED_NAME: &str = "Ajout d'un pipeline";

    fn name(raw: &str) -> ContextName {
        ContextName::parse(raw).unwrap()
    }

    #[test]
    fn a_new_folder_is_the_slug_of_the_name() {
        let folder = ContextFolder::from_name(&name(ACCENTED_NAME)).unwrap();
        assert_eq!(folder.as_str(), "ajout-d-un-pipeline");
    }

    #[test]
    fn a_legacy_folder_is_the_raw_name() {
        assert_eq!(
            ContextFolder::legacy(&name(ACCENTED_NAME)).as_str(),
            ACCENTED_NAME
        );
    }

    #[test]
    fn a_stored_folder_is_kept_as_stored() {
        assert_eq!(
            ContextFolder::from_stored(" old-folder ").unwrap().as_str(),
            "old-folder"
        );
    }

    #[test]
    fn refuses_a_blank_stored_folder() {
        assert_eq!(
            ContextFolder::from_stored(" "),
            Err(WorkspaceError::EmptyContextFolder {
                name: " ".to_string()
            })
        );
    }

    #[test]
    fn refuses_a_name_without_letters_or_digits() {
        assert_eq!(
            ContextFolder::from_name(&name("!!!")),
            Err(WorkspaceError::EmptyContextFolder {
                name: "!!!".to_string()
            })
        );
    }

    #[test]
    fn folders_are_the_same_ignoring_case() {
        let legacy = ContextFolder::legacy(&name("My-Feature"));
        let slugged = ContextFolder::from_name(&name("my feature")).unwrap();
        assert!(legacy.is_same_as(&slugged));
    }
}
