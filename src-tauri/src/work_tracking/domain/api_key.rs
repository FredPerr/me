use std::fmt;

use super::error::{InputField, InvalidInputReason, WorkTrackingError};

const MAX_API_KEY_LENGTH: usize = 512;

/// Provider API key. Deliberately has no `Serialize`, `Deserialize` or
/// `Display`, and a redacted `Debug`, so it cannot leave the backend by accident.
#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(mut raw: String) -> Result<Self, WorkTrackingError> {
        let end = raw.trim_end().len();
        let start = end - raw[..end].trim_start().len();
        let trimmed = &raw[start..end];
        let invalid = |reason| WorkTrackingError::invalid_input(InputField::ApiKey, reason);
        if trimmed.is_empty() {
            return Err(invalid(InvalidInputReason::Required));
        }
        if trimmed.chars().count() > MAX_API_KEY_LENGTH {
            return Err(invalid(InvalidInputReason::TooLong));
        }
        if trimmed
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
        {
            return Err(invalid(InvalidInputReason::InvalidFormat));
        }
        // Trim in place so the secret is not copied into a second buffer.
        raw.truncate(end);
        raw.drain(..start);
        Ok(Self(raw))
    }

    /// Only infra (HTTP client, keychain store) may read the raw key.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ApiKey(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(raw: &str) -> InvalidInputReason {
        match ApiKey::new(raw.to_string()) {
            Err(WorkTrackingError::InvalidInput {
                field: InputField::ApiKey,
                reason,
            }) => reason,
            other => panic!("expected invalid key, got {other:?}"),
        }
    }

    #[test]
    fn debug_output_is_redacted() {
        let key = ApiKey::new("super-secret-key".to_string()).unwrap();
        let printed = format!("{key:?}");
        assert!(!printed.contains("super-secret-key"));
        assert_eq!(printed, "ApiKey(***)");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let key = ApiKey::new("  twp_abc123 \n".to_string()).unwrap();
        assert_eq!(key.expose(), "twp_abc123");
    }

    #[test]
    fn rejects_invalid_keys() {
        assert_eq!(reason(""), InvalidInputReason::Required);
        assert_eq!(reason("   "), InvalidInputReason::Required);
        assert_eq!(reason("has space"), InvalidInputReason::InvalidFormat);
        assert_eq!(reason("has\ttab"), InvalidInputReason::InvalidFormat);
        assert_eq!(reason("nul\u{0}char"), InvalidInputReason::InvalidFormat);
        assert!(ApiKey::new("a".repeat(MAX_API_KEY_LENGTH)).is_ok());
        assert_eq!(
            reason(&"a".repeat(MAX_API_KEY_LENGTH + 1)),
            InvalidInputReason::TooLong
        );
    }
}
