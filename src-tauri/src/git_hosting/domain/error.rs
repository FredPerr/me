/// Input that failed validation. Mirrors the wire message of `invalidInput` errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    /// The Git host does not sign in this way, e.g. a browser sign-in for Bitbucket.
    Host,
    Username,
    Token,
    DeviceCode,
}

/// Closed set of failures of the Git Hosting context. No variant carries a
/// free-form string, so response bodies and secrets can never leak through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHostingError {
    NotConnected,
    InvalidInput { field: InputField },
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited { retry_after_seconds: Option<u32> },
    Network,
    Timeout,
    ProviderError { status: Option<u16> },
    InvalidResponse,
    Storage,
}

impl GitHostingError {
    pub fn invalid_input(field: InputField) -> Self {
        Self::InvalidInput { field }
    }
}
