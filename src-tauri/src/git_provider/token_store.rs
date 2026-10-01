//! Git provider access tokens, stored in the OS keychain via [`crate::secret_store`].

use crate::secret_store;

const KEYRING_SERVICE: &str = "me.git-provider";

/// Persist an access token for the given provider, replacing any existing one.
pub fn save_token(provider_id: &str, token: &str) -> Result<(), String> {
    secret_store::save(KEYRING_SERVICE, provider_id, token)
}

/// Read the access token for the given provider, if one is stored.
pub fn read_token(provider_id: &str) -> Result<Option<String>, String> {
    secret_store::read(KEYRING_SERVICE, provider_id)
}

/// Remove the stored access token for the given provider. Missing tokens are
/// treated as success so disconnect is idempotent.
pub fn delete_token(provider_id: &str) -> Result<(), String> {
    secret_store::delete(KEYRING_SERVICE, provider_id)
}

#[cfg(test)]
mod tests {
    use super::KEYRING_SERVICE;

    #[test]
    fn keeps_existing_github_keychain_location() {
        assert_eq!(KEYRING_SERVICE, "me.git-provider");
        assert_eq!(super::super::PROVIDER_GITHUB, "github");
    }
}
