use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::WorkspaceError;

/// Generates a `String` newtype identifier serialized as a bare string, with a
/// `parse` that refuses blank values.
macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn parse(raw: &str) -> Result<Self, WorkspaceError> {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Err(WorkspaceError::EmptyIdentifier);
                }
                Ok(Self(trimmed.to_string()))
            }

            pub fn generate() -> Self {
                Self(uuid::Uuid::new_v4().to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

typed_id!(ContextId);
typed_id!(RepositoryId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trims_the_identifier() {
        assert_eq!(
            ContextId::parse("  context-1 ").unwrap().as_str(),
            "context-1"
        );
    }

    #[test]
    fn parse_refuses_a_blank_identifier() {
        assert_eq!(
            RepositoryId::parse("   "),
            Err(WorkspaceError::EmptyIdentifier)
        );
    }

    #[test]
    fn generated_identifiers_are_distinct() {
        assert_ne!(ContextId::generate(), ContextId::generate());
    }

    #[test]
    fn serializes_as_a_bare_string() {
        let id = ContextId::parse("context-1").unwrap();
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"context-1\"");
    }
}
