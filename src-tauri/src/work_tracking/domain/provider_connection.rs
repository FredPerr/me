use super::base_url::BaseUrl;
use super::error::{InputField, InvalidInputReason, WorkTrackingError};
use super::identifiers::ConnectionId;
use super::provider_kind::ProviderKind;

const MAX_DISPLAY_NAME_LENGTH: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayName(String);

impl DisplayName {
    /// Empty or missing input defaults to the site host. The host default is
    /// always accepted, even when it is longer than the user-input limit.
    pub fn parse(raw: Option<&str>, base_url: &BaseUrl) -> Result<Self, WorkTrackingError> {
        let trimmed = raw.map(str::trim).unwrap_or_default();
        if trimmed.is_empty() || trimmed == base_url.host() {
            return Ok(Self(base_url.host().to_string()));
        }
        if trimmed.chars().count() > MAX_DISPLAY_NAME_LENGTH {
            return Err(WorkTrackingError::invalid_input(
                InputField::DisplayName,
                InvalidInputReason::TooLong,
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Aggregate root of the context: a configured provider site. Holds no secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConnection {
    id: ConnectionId,
    kind: ProviderKind,
    base_url: BaseUrl,
    display_name: DisplayName,
}

impl ProviderConnection {
    pub fn new(kind: ProviderKind, base_url: BaseUrl, display_name: DisplayName) -> Self {
        Self {
            id: ConnectionId::for_site(kind, &base_url),
            kind,
            base_url,
            display_name,
        }
    }

    /// Rehydrates a stored connection; returns `None` when the stored id does
    /// not match the site, so a tampered record is ignored.
    pub fn restore(
        id: ConnectionId,
        kind: ProviderKind,
        base_url: BaseUrl,
        display_name: DisplayName,
    ) -> Option<Self> {
        let connection = Self::new(kind, base_url, display_name);
        (connection.id == id).then_some(connection)
    }

    pub fn id(&self) -> &ConnectionId {
        &self.id
    }

    pub fn kind(&self) -> ProviderKind {
        self.kind
    }

    pub fn base_url(&self) -> &BaseUrl {
        &self.base_url
    }

    pub fn display_name(&self) -> &DisplayName {
        &self.display_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_url() -> BaseUrl {
        BaseUrl::parse("https://acme.teamwork.com").unwrap()
    }

    #[test]
    fn display_name_defaults_to_host() {
        assert_eq!(
            DisplayName::parse(None, &base_url()).unwrap().as_str(),
            "acme.teamwork.com"
        );
        assert_eq!(
            DisplayName::parse(Some("   "), &base_url())
                .unwrap()
                .as_str(),
            "acme.teamwork.com"
        );
        assert_eq!(
            DisplayName::parse(Some(" Acme "), &base_url())
                .unwrap()
                .as_str(),
            "Acme"
        );
    }

    #[test]
    fn display_name_rejects_too_long() {
        assert!(DisplayName::parse(Some(&"a".repeat(100)), &base_url()).is_ok());
        assert_eq!(
            DisplayName::parse(Some(&"a".repeat(101)), &base_url()),
            Err(WorkTrackingError::invalid_input(
                InputField::DisplayName,
                InvalidInputReason::TooLong
            ))
        );
    }

    #[test]
    fn display_name_accepts_long_host_default() {
        let host = format!("{}.com", "a".repeat(200));
        let long_base_url = BaseUrl::parse(&format!("https://{host}")).unwrap();
        assert_eq!(
            DisplayName::parse(Some(&host), &long_base_url)
                .unwrap()
                .as_str(),
            host
        );
    }

    #[test]
    fn new_derives_id_from_site() {
        let display_name = DisplayName::parse(None, &base_url()).unwrap();
        let connection = ProviderConnection::new(ProviderKind::Teamwork, base_url(), display_name);
        assert_eq!(connection.id().as_str(), "teamwork:acme.teamwork.com");
    }

    #[test]
    fn restore_rejects_mismatched_id() {
        let display_name = DisplayName::parse(None, &base_url()).unwrap();
        let matching =
            ConnectionId::parse("teamwork:acme.teamwork.com", InputField::ConnectionId).unwrap();
        assert!(ProviderConnection::restore(
            matching,
            ProviderKind::Teamwork,
            base_url(),
            display_name.clone()
        )
        .is_some());
        let tampered =
            ConnectionId::parse("teamwork:evil.example.com", InputField::ConnectionId).unwrap();
        assert!(ProviderConnection::restore(
            tampered,
            ProviderKind::Teamwork,
            base_url(),
            display_name
        )
        .is_none());
    }
}
