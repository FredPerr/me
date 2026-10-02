use std::collections::HashMap;
use std::sync::Arc;

use crate::work_tracking::domain::api_key::ApiKey;
use crate::work_tracking::domain::error::{InputField, InvalidInputReason, WorkTrackingError};
use crate::work_tracking::domain::ports::WorkTracker;
use crate::work_tracking::domain::provider_connection::ProviderConnection;
use crate::work_tracking::domain::provider_kind::ProviderKind;

pub type WorkTrackerFactory = Arc<
    dyn Fn(&ProviderConnection, ApiKey) -> Result<Box<dyn WorkTracker>, WorkTrackingError>
        + Send
        + Sync,
>;

/// Maps each provider kind to the factory building its adapter.
#[derive(Default)]
pub struct WorkTrackerRegistry {
    factories: HashMap<ProviderKind, WorkTrackerFactory>,
}

impl WorkTrackerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(mut self, kind: ProviderKind, factory: WorkTrackerFactory) -> Self {
        self.factories.insert(kind, factory);
        self
    }

    pub fn create(
        &self,
        connection: &ProviderConnection,
        key: ApiKey,
    ) -> Result<Box<dyn WorkTracker>, WorkTrackingError> {
        let factory = self.factories.get(&connection.kind()).ok_or_else(|| {
            WorkTrackingError::invalid_input(
                InputField::ProviderKind,
                InvalidInputReason::Unsupported,
            )
        })?;
        factory(connection, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_tracking::application::test_support::{sample_connection, Script};

    #[test]
    fn creates_through_registered_factory() {
        let script = Script::shared();
        let registry =
            WorkTrackerRegistry::new().register(ProviderKind::Teamwork, Script::factory(&script));
        let key = ApiKey::new("key-1".into()).unwrap();
        assert!(registry.create(&sample_connection(), key).is_ok());
        assert_eq!(
            script.lock().unwrap().created_with_keys,
            vec!["key-1".to_string()]
        );
    }

    #[test]
    fn unregistered_kind_is_unsupported() {
        let key = ApiKey::new("key-1".into()).unwrap();
        let result = WorkTrackerRegistry::new().create(&sample_connection(), key);
        assert_eq!(
            result.err(),
            Some(WorkTrackingError::invalid_input(
                InputField::ProviderKind,
                InvalidInputReason::Unsupported
            ))
        );
    }
}
