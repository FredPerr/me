use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::WorkspaceError;

/// The name of a context, shown on the board. Unique within a project,
/// ignoring case.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContextName(String);

impl ContextName {
    pub fn parse(raw: &str) -> Result<Self, WorkspaceError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(WorkspaceError::EmptyContextName);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_same_as(&self, other: &ContextName) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

impl fmt::Display for ContextName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trims_the_name() {
        assert_eq!(
            ContextName::parse("  My Feature ").unwrap().as_str(),
            "My Feature"
        );
    }

    #[test]
    fn parse_refuses_a_blank_name() {
        assert_eq!(
            ContextName::parse("  "),
            Err(WorkspaceError::EmptyContextName)
        );
    }

    #[test]
    fn names_are_the_same_ignoring_case() {
        let name = ContextName::parse("My Feature").unwrap();
        assert!(name.is_same_as(&ContextName::parse("MY FEATURE").unwrap()));
        assert!(!name.is_same_as(&ContextName::parse("My Feature 2").unwrap()));
    }
}
