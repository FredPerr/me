use serde::Serialize;

use super::base_url::BaseUrl;
use super::error::{InputField, InvalidInputReason, WorkTrackingError};
use super::provider_kind::ProviderKind;

/// Shared by every typed ID; fits the longest `ConnectionId::for_site` output
/// (9-char `"{kind}:"` prefix + 253-char host + `":65535"` = 268 chars).
pub const MAX_ID_LENGTH: usize = 300;

/// Generates a `String` newtype identifier serialized as a bare string.
///
/// `typed_id!(Name)` declares the type; `typed_id!(@parse Name)` adds a
/// validating `parse` for IDs that arrive from the frontend, and
/// `typed_id!(@numeric Name)` adds an infallible `from_numeric` for adapters.
///
/// Provider adapters must map remote identifiers to slash-free values (for
/// example the GitHub numeric repository id, not `owner/repo`), because IDs are
/// used as URL path segments and keychain accounts.
macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            // WorkItemId and PersonId are only serialized, never read back in non-test code.
            #[allow(dead_code)]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
    (@parse $name:ident) => {
        impl $name {
            pub fn parse(raw: &str, field: InputField) -> Result<Self, WorkTrackingError> {
                parse_identifier(raw, field).map(Self)
            }
        }
    };
    (@numeric $name:ident) => {
        impl $name {
            pub fn from_numeric(value: u64) -> Self {
                Self(value.to_string())
            }
        }
    };
}

typed_id!(ConnectionId);
typed_id!(@parse ConnectionId);
typed_id!(WorkProjectId);
typed_id!(@parse WorkProjectId);
typed_id!(@numeric WorkProjectId);
typed_id!(WorkItemGroupId);
typed_id!(@parse WorkItemGroupId);
typed_id!(@numeric WorkItemGroupId);
typed_id!(WorkItemId);
typed_id!(@numeric WorkItemId);
typed_id!(PersonId);
typed_id!(@numeric PersonId);

fn parse_identifier(raw: &str, field: InputField) -> Result<String, WorkTrackingError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(WorkTrackingError::invalid_input(
            field,
            InvalidInputReason::Required,
        ));
    }
    if trimmed.chars().count() > MAX_ID_LENGTH {
        return Err(WorkTrackingError::invalid_input(
            field,
            InvalidInputReason::TooLong,
        ));
    }
    let is_forbidden =
        |character: char| matches!(character, '/' | '?' | '#') || character.is_control();
    if trimmed.chars().any(is_forbidden) {
        return Err(WorkTrackingError::invalid_input(
            field,
            InvalidInputReason::InvalidFormat,
        ));
    }
    Ok(trimmed.to_string())
}

impl ConnectionId {
    /// Deterministic id `"{kind}:{authority}"`: re-saving the same site upserts.
    /// Inputs are already validated, so `parse` is skipped.
    pub fn for_site(kind: ProviderKind, base_url: &BaseUrl) -> Self {
        Self(format!("{}:{}", kind.as_str(), base_url.authority()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(raw: &str) -> InvalidInputReason {
        match WorkProjectId::parse(raw, InputField::ProjectId) {
            Err(WorkTrackingError::InvalidInput {
                field: InputField::ProjectId,
                reason,
            }) => reason,
            other => panic!("expected invalid id for {raw:?}, got {other:?}"),
        }
    }

    #[test]
    fn parse_trims() {
        let id = WorkProjectId::parse("  42 ", InputField::ProjectId).unwrap();
        assert_eq!(id.as_str(), "42");
    }

    #[test]
    fn parse_rejects_empty_and_forbidden_characters() {
        assert_eq!(reason(""), InvalidInputReason::Required);
        assert_eq!(reason("  "), InvalidInputReason::Required);
        for raw in ["owner/repo", "a?b", "a#b", "a\u{0}b", "a\nb"] {
            assert_eq!(reason(raw), InvalidInputReason::InvalidFormat, "{raw:?}");
        }
    }

    #[test]
    fn parse_enforces_max_length() {
        assert!(WorkProjectId::parse(&"a".repeat(MAX_ID_LENGTH), InputField::ProjectId).is_ok());
        assert_eq!(
            reason(&"a".repeat(MAX_ID_LENGTH + 1)),
            InvalidInputReason::TooLong
        );
    }

    #[test]
    fn parse_reports_the_given_field() {
        assert_eq!(
            ConnectionId::parse("", InputField::ConnectionId),
            Err(WorkTrackingError::invalid_input(
                InputField::ConnectionId,
                InvalidInputReason::Required
            ))
        );
        assert_eq!(
            WorkItemGroupId::parse("a/b", InputField::GroupId),
            Err(WorkTrackingError::invalid_input(
                InputField::GroupId,
                InvalidInputReason::InvalidFormat
            ))
        );
    }

    #[test]
    fn from_numeric_handles_max_value() {
        assert_eq!(
            WorkItemId::from_numeric(u64::MAX).as_str(),
            "18446744073709551615"
        );
        assert_eq!(PersonId::from_numeric(7).as_str(), "7");
    }

    #[test]
    fn for_site_round_trips_for_longest_authority() {
        let host = format!("{}.com", "a".repeat(249));
        assert_eq!(host.len(), 253);
        let base_url = BaseUrl::parse(&format!("https://{host}:65535")).unwrap();
        let id = ConnectionId::for_site(ProviderKind::Teamwork, &base_url);
        assert_eq!(id.as_str().len(), 268);
        let reparsed = ConnectionId::parse(id.as_str(), InputField::ConnectionId).unwrap();
        assert_eq!(reparsed, id);
    }

    #[test]
    fn for_site_uses_kind_and_authority() {
        let base_url = BaseUrl::parse("https://acme.teamwork.com").unwrap();
        assert_eq!(
            ConnectionId::for_site(ProviderKind::Teamwork, &base_url).as_str(),
            "teamwork:acme.teamwork.com"
        );
    }
}
