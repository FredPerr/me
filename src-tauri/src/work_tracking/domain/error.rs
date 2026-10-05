/// Input that failed validation. Mirrors the wire `field` of `invalidInput` errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    ProviderKind,
    BaseUrl,
    ApiKey,
    DisplayName,
    ConnectionId,
    ProjectId,
    GroupId,
    ItemId,
    Cursor,
    PageSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidInputReason {
    Required,
    TooLong,
    InvalidFormat,
    HttpsRequired,
    UnsupportedComponent,
    OutOfRange,
    Unsupported,
}

/// Closed set of failures of the Work Tracking context. No variant carries a
/// free-form string, so provider or credential data can never leak through it.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkTrackingError {
    NotConfigured,
    InvalidInput {
        field: InputField,
        reason: InvalidInputReason,
    },
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited {
        retry_after_seconds: Option<u64>,
    },
    Timeout,
    Network,
    ProviderError {
        status: Option<u16>,
    },
    InvalidResponse,
    StorageError,
}

impl WorkTrackingError {
    pub fn invalid_input(field: InputField, reason: InvalidInputReason) -> Self {
        Self::InvalidInput { field, reason }
    }
}
