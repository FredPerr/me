use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::WorkspaceError;
use super::identifiers::RepositoryId;

/// The short name that identifies a Project. A project without configured
/// repositories uses it as the id of its single repository at the root.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectTag(String);

impl ProjectTag {
    pub fn parse(raw: &str) -> Result<Self, WorkspaceError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(WorkspaceError::EmptyProjectTag);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn root_repository_id(&self) -> RepositoryId {
        RepositoryId::parse(&self.0).expect("a project tag is never blank")
    }
}

impl fmt::Display for ProjectTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trims_the_tag() {
        assert_eq!(
            ProjectTag::parse(" my-project ").unwrap().as_str(),
            "my-project"
        );
    }

    #[test]
    fn parse_refuses_a_blank_tag() {
        assert_eq!(ProjectTag::parse(""), Err(WorkspaceError::EmptyProjectTag));
    }

    #[test]
    fn the_root_repository_id_is_the_tag() {
        let tag = ProjectTag::parse("my-project").unwrap();
        assert_eq!(tag.root_repository_id().as_str(), "my-project");
    }
}
