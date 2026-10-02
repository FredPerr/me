//! Secure secret storage backed by the OS keychain, keyed by (service, account).
//!
//! We deliberately avoid `tauri-plugin-store` for secrets: that plugin writes
//! plaintext JSON to the app-data directory, which is unsuitable for OAuth
//! credentials or API keys. The `keyring` crate maps to Keychain (macOS), the
//! Windows Credential Manager, and the Secret Service (Linux).

fn entry(service: &str, account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(service, account).map_err(|error| error.to_string())
}

/// Persist a secret for the given service and account, replacing any existing one.
pub fn save(service: &str, account: &str, secret: &str) -> Result<(), String> {
    entry(service, account)?
        .set_password(secret)
        .map_err(|error| error.to_string())
}

/// Read the secret for the given service and account, if one is stored.
pub fn read(service: &str, account: &str) -> Result<Option<String>, String> {
    match entry(service, account)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// Remove the stored secret. Missing entries are treated as success so
/// deletion is idempotent.
pub fn delete(service: &str, account: &str) -> Result<(), String> {
    match entry(service, account)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}
