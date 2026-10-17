use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::work_tracking::domain::error::WorkTrackingError;
use crate::work_tracking::domain::identifiers::ConnectionId;
use crate::work_tracking::domain::ports::ConnectionRepository;
use crate::work_tracking::domain::provider_connection::ProviderConnection;

/// Connections kept in memory, keyed by connection id like redb. Used by the
/// contract tests and the one-time import's unit tests.
#[derive(Default)]
pub struct InMemoryConnectionRepository {
    connections: Mutex<BTreeMap<String, ProviderConnection>>,
}

impl InMemoryConnectionRepository {
    fn connections(&self) -> MutexGuard<'_, BTreeMap<String, ProviderConnection>> {
        // Every write replaces whole connections, so a poisoned map is still consistent.
        self.connections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl ConnectionRepository for InMemoryConnectionRepository {
    fn list(&self) -> Result<Vec<ProviderConnection>, WorkTrackingError> {
        Ok(self.connections().values().cloned().collect())
    }

    fn find(&self, id: &ConnectionId) -> Result<Option<ProviderConnection>, WorkTrackingError> {
        Ok(self.connections().get(id.as_str()).cloned())
    }

    fn save(&self, connection: &ProviderConnection) -> Result<(), WorkTrackingError> {
        self.connections()
            .insert(connection.id().as_str().to_string(), connection.clone());
        Ok(())
    }

    fn remove(&self, id: &ConnectionId) -> Result<(), WorkTrackingError> {
        self.connections().remove(id.as_str());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_tracking::infra::connection_repository_contract::connection_repository_contract_tests;

    fn with_repository(test: fn(&dyn ConnectionRepository)) {
        test(&InMemoryConnectionRepository::default());
    }

    connection_repository_contract_tests!(with_repository);
}
