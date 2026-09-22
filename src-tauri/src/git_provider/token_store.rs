//! Secure access-token storage backed by the OS keychain.
//!
//! We deliberately avoid `tauri-plugin-store` for tokens: that plugin writes
//! plaintext JSON to the app-data directory, which is unsuitable for OAuth
//! credentials. The `keyring` crate maps to Keychain (macOS), the Windows
//! Credential Manager, and the Secret Service (Linux).

const KEYRING_SERVICE: &str = "me.git-provider";

fn entry(provider_id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, provider_id).map_err(|error| error.to_string())
}

/// Persist an access token for the given provider, replacing any existing one.
pub fn save_token(provider_id: &str, token: &str) -> Result<(), String> {
    entry(provider_id)?
        .set_password(token)
        .map_err(|error| error.to_string())
}

/// Read the access token for the given provider, if one is stored.
pub fn read_token(provider_id: &str) -> Result<Option<String>, String> {
    match entry(provider_id)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// Remove the stored access token for the given provider. Missing tokens are
/// treated as success so disconnect is idempotent.
pub fn delete_token(provider_id: &str) -> Result<(), String> {
    match entry(provider_id)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}
