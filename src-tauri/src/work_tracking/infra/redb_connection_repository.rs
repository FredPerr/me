use std::fmt::Display;
use std::sync::Arc;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::work_tracking::domain::base_url::BaseUrl;
use crate::work_tracking::domain::error::{InputField, WorkTrackingError};
use crate::work_tracking::domain::identifiers::ConnectionId;
use crate::work_tracking::domain::ports::ConnectionRepository;
use crate::work_tracking::domain::provider_connection::{DisplayName, ProviderConnection};
use crate::work_tracking::domain::provider_kind::ProviderKind;

/// One record per connection, keyed by its id, holding the metadata (never the
/// API key) as MessagePack with field names so added fields still read from
/// older records.
const CONNECTIONS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("work_tracking_connections");

/// Stored connection metadata. Holds no secret.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionRecord {
    pub id: String,
    pub kind: String,
    pub base_url: String,
    pub display_name: String,
}

impl ConnectionRecord {
    pub fn from_connection(connection: &ProviderConnection) -> Self {
        Self {
            id: connection.id().as_str().to_string(),
            kind: connection.kind().as_str().to_string(),
            base_url: connection.base_url().as_str().to_string(),
            display_name: connection.display_name().as_str().to_string(),
        }
    }

    /// Rehydrates the connection, dropping a record whose id no longer matches
    /// its site (tampered) or whose fields no longer validate.
    pub fn to_connection(&self) -> Option<ProviderConnection> {
        let id = ConnectionId::parse(&self.id, InputField::ConnectionId).ok()?;
        let kind = ProviderKind::parse(&self.kind).ok()?;
        let base_url = BaseUrl::parse(&self.base_url).ok()?;
        let display_name = DisplayName::parse(Some(&self.display_name), &base_url).ok()?;
        ProviderConnection::restore(id, kind, base_url, display_name)
    }
}

fn storage_error(error: impl Display) -> WorkTrackingError {
    eprintln!("[work_tracking] connection storage failed: {error}");
    WorkTrackingError::StorageError
}

/// An unreadable or tampered record reads as absent, so it is replaced on the
/// next save rather than failing the call.
fn decode(key: &str, bytes: &[u8]) -> Option<ProviderConnection> {
    match rmp_serde::from_slice::<ConnectionRecord>(bytes) {
        Ok(record) => record.to_connection(),
        Err(error) => {
            eprintln!("[work_tracking] connection record '{key}' unreadable: {error}");
            None
        }
    }
}

/// Connection metadata in the shared redb database.
pub struct RedbConnectionRepository {
    database: Arc<Database>,
}

impl RedbConnectionRepository {
    /// Uses the shared database, creating its `work_tracking_connections` table
    /// when missing.
    pub fn new(database: Arc<Database>) -> Result<Self, WorkTrackingError> {
        let transaction = database.begin_write().map_err(storage_error)?;
        transaction
            .open_table(CONNECTIONS_TABLE)
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(Self { database })
    }

    fn write(
        &self,
        apply: impl FnOnce(&mut redb::Table<&str, &[u8]>) -> Result<(), WorkTrackingError>,
    ) -> Result<(), WorkTrackingError> {
        let transaction = self.database.begin_write().map_err(storage_error)?;
        {
            let mut table = transaction
                .open_table(CONNECTIONS_TABLE)
                .map_err(storage_error)?;
            apply(&mut table)?;
        }
        transaction.commit().map_err(storage_error)
    }
}

impl ConnectionRepository for RedbConnectionRepository {
    fn list(&self) -> Result<Vec<ProviderConnection>, WorkTrackingError> {
        let transaction = self.database.begin_read().map_err(storage_error)?;
        let table = transaction
            .open_table(CONNECTIONS_TABLE)
            .map_err(storage_error)?;
        let mut connections = Vec::new();
        for entry in table.iter().map_err(storage_error)? {
            let (key, value) = entry.map_err(storage_error)?;
            connections.extend(decode(key.value(), value.value()));
        }
        Ok(connections)
    }

    fn find(&self, id: &ConnectionId) -> Result<Option<ProviderConnection>, WorkTrackingError> {
        let transaction = self.database.begin_read().map_err(storage_error)?;
        let table = transaction
            .open_table(CONNECTIONS_TABLE)
            .map_err(storage_error)?;
        let record = table.get(id.as_str()).map_err(storage_error)?;
        Ok(record.and_then(|record| decode(id.as_str(), record.value())))
    }

    fn save(&self, connection: &ProviderConnection) -> Result<(), WorkTrackingError> {
        let record = rmp_serde::to_vec_named(&ConnectionRecord::from_connection(connection))
            .map_err(storage_error)?;
        self.write(|table| {
            table
                .insert(connection.id().as_str(), record.as_slice())
                .map_err(storage_error)?;
            Ok(())
        })
    }

    fn remove(&self, id: &ConnectionId) -> Result<(), WorkTrackingError> {
        self.write(|table| {
            table.remove(id.as_str()).map_err(storage_error)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::storage;
    use crate::work_tracking::infra::connection_repository_contract::connection_repository_contract_tests;

    const ACME_ID: &str = "teamwork:acme.teamwork.com";
    const CORRUPT_BYTES: &[u8] = b"not a connection";

    fn repository_in(directory: &TempDir) -> RedbConnectionRepository {
        RedbConnectionRepository::new(storage::open(directory.path()).unwrap()).unwrap()
    }

    fn with_repository(test: fn(&dyn ConnectionRepository)) {
        let directory = tempfile::tempdir().unwrap();
        test(&repository_in(&directory));
    }

    connection_repository_contract_tests!(with_repository);

    fn acme_connection() -> ProviderConnection {
        let base_url = BaseUrl::parse("https://acme.teamwork.com").unwrap();
        let display_name = DisplayName::parse(None, &base_url).unwrap();
        ProviderConnection::new(ProviderKind::Teamwork, base_url, display_name)
    }

    fn write_raw_record(repository: &RedbConnectionRepository, key: &str, bytes: &[u8]) {
        repository
            .write(|table| {
                table.insert(key, bytes).map_err(storage_error)?;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn reopening_the_database_keeps_the_connections() {
        let directory = tempfile::tempdir().unwrap();
        repository_in(&directory).save(&acme_connection()).unwrap();

        let reopened = repository_in(&directory);

        assert_eq!(reopened.list().unwrap(), vec![acme_connection()]);
    }

    #[test]
    fn stores_the_connection_with_field_names() {
        let directory = tempfile::tempdir().unwrap();
        let repository = repository_in(&directory);
        let record = rmp_serde::to_vec_named(&json!({
            "id": ACME_ID,
            "kind": "teamwork",
            "baseUrl": "https://acme.teamwork.com",
            "displayName": "acme.teamwork.com",
        }))
        .unwrap();

        write_raw_record(&repository, ACME_ID, &record);

        assert_eq!(repository.list().unwrap(), vec![acme_connection()]);
    }

    #[test]
    fn an_unreadable_record_reads_as_absent() {
        let directory = tempfile::tempdir().unwrap();
        let repository = repository_in(&directory);

        write_raw_record(&repository, ACME_ID, CORRUPT_BYTES);

        assert_eq!(repository.list().unwrap(), vec![]);
    }

    #[test]
    fn a_tampered_id_reads_as_absent() {
        let directory = tempfile::tempdir().unwrap();
        let repository = repository_in(&directory);
        let record = rmp_serde::to_vec_named(&json!({
            "id": "teamwork:evil.example.com",
            "kind": "teamwork",
            "baseUrl": "https://acme.teamwork.com",
            "displayName": "Tampered",
        }))
        .unwrap();

        write_raw_record(&repository, "teamwork:evil.example.com", &record);

        assert_eq!(repository.list().unwrap(), vec![]);
    }
}
