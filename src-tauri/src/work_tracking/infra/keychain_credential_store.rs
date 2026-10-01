use crate::secret_store;
use crate::work_tracking::domain::api_key::ApiKey;
use crate::work_tracking::domain::error::WorkTrackingError;
use crate::work_tracking::domain::identifiers::ConnectionId;
use crate::work_tracking::domain::ports::CredentialStore;

const SERVICE: &str = "me.work-tracking";

/// API keys in the OS keychain, one entry per connection id. Keyring error
/// strings are dropped so nothing platform-specific reaches the UI.
pub struct KeychainCredentialStore;

impl CredentialStore for KeychainCredentialStore {
    fn save(&self, id: &ConnectionId, key: &ApiKey) -> Result<(), WorkTrackingError> {
        secret_store::save(SERVICE, id.as_str(), key.expose())
            .map_err(|_| WorkTrackingError::StorageError)
    }

    fn read(&self, id: &ConnectionId) -> Result<Option<ApiKey>, WorkTrackingError> {
        let secret = secret_store::read(SERVICE, id.as_str())
            .map_err(|_| WorkTrackingError::StorageError)?;
        // An entry that no longer passes validation is treated as missing, so
        // the user is offered to replace it.
        Ok(secret.and_then(|value| ApiKey::new(value).ok()))
    }

    fn delete(&self, id: &ConnectionId) -> Result<(), WorkTrackingError> {
        secret_store::delete(SERVICE, id.as_str()).map_err(|_| WorkTrackingError::StorageError)
    }
}

#[cfg(test)]
mod tests {
    use super::SERVICE;

    #[test]
    fn uses_dedicated_keychain_service() {
        assert_eq!(SERVICE, "me.work-tracking");
    }
}
