use crate::git_hosting::domain::host_kind::HostKind;
use crate::git_hosting::domain::host_secret::HostSecret;
use crate::git_hosting::domain::ports::{HostTokens, HostTokensUnavailable};
use crate::secret_store;

/// Kept from the old provider module so existing sign-ins keep working.
const SERVICE: &str = "me.git-provider";

/// Git host secrets in the OS keychain, one entry per Git host.
pub struct KeychainHostTokens;

impl HostTokens for KeychainHostTokens {
    fn read(&self, host: HostKind) -> Result<Option<HostSecret>, HostTokensUnavailable> {
        let secret =
            secret_store::read(SERVICE, host.as_str()).map_err(|_| HostTokensUnavailable)?;
        Ok(secret.map(HostSecret::new))
    }

    fn save(&self, host: HostKind, secret: &HostSecret) -> Result<(), HostTokensUnavailable> {
        secret_store::save(SERVICE, host.as_str(), secret.expose())
            .map_err(|_| HostTokensUnavailable)
    }

    fn delete(&self, host: HostKind) -> Result<(), HostTokensUnavailable> {
        secret_store::delete(SERVICE, host.as_str()).map_err(|_| HostTokensUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_existing_keychain_location() {
        assert_eq!(SERVICE, "me.git-provider");
        assert_eq!(HostKind::Github.as_str(), "github");
        assert_eq!(HostKind::Bitbucket.as_str(), "bitbucket");
    }
}
