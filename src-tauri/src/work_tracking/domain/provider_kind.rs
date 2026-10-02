use serde::Serialize;

use super::error::{InputField, InvalidInputReason, WorkTrackingError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    Teamwork,
}

impl ProviderKind {
    pub fn parse(raw: &str) -> Result<Self, WorkTrackingError> {
        match raw {
            "teamwork" => Ok(Self::Teamwork),
            _ => Err(WorkTrackingError::invalid_input(
                InputField::ProviderKind,
                InvalidInputReason::Unsupported,
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Teamwork => "teamwork",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_kind_exactly() {
        assert_eq!(ProviderKind::parse("teamwork"), Ok(ProviderKind::Teamwork));
        assert_eq!(ProviderKind::Teamwork.as_str(), "teamwork");
        for raw in ["Teamwork", " teamwork", "jira", ""] {
            assert_eq!(
                ProviderKind::parse(raw),
                Err(WorkTrackingError::invalid_input(
                    InputField::ProviderKind,
                    InvalidInputReason::Unsupported
                ))
            );
        }
    }
}
