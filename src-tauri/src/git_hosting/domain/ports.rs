use super::host_kind::HostKind;
use super::host_secret::HostSecret;

/// The secret store could not be used. Platform details are dropped so they
/// never reach the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostTokensUnavailable;

/// Each Git host's secret, at most one per host.
pub trait HostTokens: Send + Sync {
    fn read(&self, host: HostKind) -> Result<Option<HostSecret>, HostTokensUnavailable>;
    fn save(&self, host: HostKind, secret: &HostSecret) -> Result<(), HostTokensUnavailable>;
    /// Succeeds when no secret is stored.
    fn delete(&self, host: HostKind) -> Result<(), HostTokensUnavailable>;
}
