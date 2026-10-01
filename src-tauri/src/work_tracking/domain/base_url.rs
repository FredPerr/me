use super::error::{InputField, InvalidInputReason, WorkTrackingError};

const MAX_BASE_URL_LENGTH: usize = 2048;
const MAX_HOST_LENGTH: usize = 253;
const DEFAULT_HTTPS_PORT: u16 = 443;

/// Normalized `https://{host}[:{port}]` site URL. Parsed with a deliberately
/// narrow grammar: anything outside it is rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseUrl {
    normalized: String,
    host: String,
    port: Option<u16>,
}

fn invalid(reason: InvalidInputReason) -> WorkTrackingError {
    WorkTrackingError::invalid_input(InputField::BaseUrl, reason)
}

impl BaseUrl {
    pub fn parse(raw: &str) -> Result<Self, WorkTrackingError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(invalid(InvalidInputReason::Required));
        }
        if trimmed.chars().count() > MAX_BASE_URL_LENGTH {
            return Err(invalid(InvalidInputReason::TooLong));
        }

        let (scheme, rest) = trimmed
            .split_once("://")
            .ok_or_else(|| invalid(InvalidInputReason::InvalidFormat))?;
        if !scheme.eq_ignore_ascii_case("https") {
            return Err(invalid(InvalidInputReason::HttpsRequired));
        }

        let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, remainder) = rest.split_at(authority_end);
        if authority.contains('@') {
            return Err(invalid(InvalidInputReason::UnsupportedComponent));
        }

        let (raw_host, raw_port) = match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        };
        let host = parse_host(raw_host)?;
        let port = raw_port.map(parse_port).transpose()?;
        let port = port.filter(|value| *value != DEFAULT_HTTPS_PORT);

        if !remainder.chars().all(|character| character == '/') {
            return Err(invalid(InvalidInputReason::UnsupportedComponent));
        }

        let normalized = match port {
            Some(port) => format!("https://{host}:{port}"),
            None => format!("https://{host}"),
        };
        Ok(Self {
            normalized,
            host,
            port,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.normalized
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn authority(&self) -> String {
        match self.port {
            Some(port) => format!("{}:{port}", self.host),
            None => self.host.clone(),
        }
    }
}

fn parse_host(raw_host: &str) -> Result<String, WorkTrackingError> {
    let host = raw_host.to_ascii_lowercase();
    if host.is_empty() {
        return Err(invalid(InvalidInputReason::InvalidFormat));
    }
    if host.len() > MAX_HOST_LENGTH {
        return Err(invalid(InvalidInputReason::TooLong));
    }
    let allowed = |character: char| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '-')
    };
    if !host.chars().all(allowed) || host.split('.').any(str::is_empty) {
        return Err(invalid(InvalidInputReason::InvalidFormat));
    }
    // Provider sites are hostnames: an all-digit last label means an IPv4
    // address or a numeric TLD.
    let last_label = host.rsplit('.').next().unwrap_or_default();
    if last_label
        .chars()
        .all(|character| character.is_ascii_digit())
    {
        return Err(invalid(InvalidInputReason::InvalidFormat));
    }
    Ok(host)
}

fn parse_port(raw_port: &str) -> Result<u16, WorkTrackingError> {
    if raw_port.is_empty() || !raw_port.chars().all(|character| character.is_ascii_digit()) {
        return Err(invalid(InvalidInputReason::InvalidFormat));
    }
    match raw_port.parse::<u16>() {
        Ok(port) if port >= 1 => Ok(port),
        _ => Err(invalid(InvalidInputReason::InvalidFormat)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(raw: &str) -> InvalidInputReason {
        match BaseUrl::parse(raw) {
            Err(WorkTrackingError::InvalidInput {
                field: InputField::BaseUrl,
                reason,
            }) => reason,
            other => panic!("expected invalid base url for {raw}, got {other:?}"),
        }
    }

    #[test]
    fn requires_https() {
        assert_eq!(
            reason("http://acme.teamwork.com"),
            InvalidInputReason::HttpsRequired
        );
        assert_eq!(
            reason("ftp://acme.teamwork.com"),
            InvalidInputReason::HttpsRequired
        );
        assert_eq!(
            reason("acme.teamwork.com"),
            InvalidInputReason::InvalidFormat
        );
    }

    #[test]
    fn normalizes_case_and_trailing_slashes() {
        let base_url = BaseUrl::parse("  HTTPS://ACME.teamwork.com///  ").unwrap();
        assert_eq!(base_url.as_str(), "https://acme.teamwork.com");
        assert_eq!(base_url.host(), "acme.teamwork.com");
        assert_eq!(base_url.authority(), "acme.teamwork.com");
    }

    #[test]
    fn drops_default_port_and_keeps_others() {
        assert_eq!(
            BaseUrl::parse("https://acme.teamwork.com:443")
                .unwrap()
                .as_str(),
            "https://acme.teamwork.com"
        );
        let custom = BaseUrl::parse("https://acme.teamwork.com:8443/").unwrap();
        assert_eq!(custom.as_str(), "https://acme.teamwork.com:8443");
        assert_eq!(custom.authority(), "acme.teamwork.com:8443");
    }

    #[test]
    fn rejects_invalid_ports() {
        for raw in [
            "https://acme.teamwork.com:",
            "https://acme.teamwork.com:0",
            "https://acme.teamwork.com:65536",
            "https://acme.teamwork.com:+1",
        ] {
            assert_eq!(reason(raw), InvalidInputReason::InvalidFormat, "{raw}");
        }
    }

    #[test]
    fn rejects_unsupported_components() {
        for raw in [
            "https://user:pass@acme.teamwork.com",
            "https://acme.teamwork.com?query=1",
            "https://acme.teamwork.com/?query=1",
            "https://acme.teamwork.com#fragment",
            "https://acme.teamwork.com/app",
        ] {
            assert_eq!(
                reason(raw),
                InvalidInputReason::UnsupportedComponent,
                "{raw}"
            );
        }
    }

    #[test]
    fn rejects_empty_and_too_long() {
        assert_eq!(reason(""), InvalidInputReason::Required);
        assert_eq!(reason("   "), InvalidInputReason::Required);
        let too_long = format!("https://{}.com", "a".repeat(2048));
        assert_eq!(reason(&too_long), InvalidInputReason::TooLong);
        let long_host = format!("https://{}.com", "a".repeat(250));
        assert_eq!(reason(&long_host), InvalidInputReason::TooLong);
    }

    #[test]
    fn rejects_ip_addresses_and_invalid_hosts() {
        for raw in [
            "https://192.168.0.1",
            "https://acme.123",
            "https://[::1]",
            "https://acme..teamwork.com",
            "https://.acme.com",
            "https://acme_site.com",
            "https://",
        ] {
            assert_eq!(reason(raw), InvalidInputReason::InvalidFormat, "{raw}");
        }
    }
}
