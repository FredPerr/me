use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::work_tracking::domain::base_url::BaseUrl;
use crate::work_tracking::domain::error::{InputField, WorkTrackingError};
use crate::work_tracking::domain::identifiers::ConnectionId;
use crate::work_tracking::domain::ports::ConnectionRepository;
use crate::work_tracking::domain::provider_connection::{DisplayName, ProviderConnection};
use crate::work_tracking::domain::provider_kind::ProviderKind;

const STORE_FILE: &str = "work-tracking.json";
const CONNECTIONS_KEY: &str = "connections";
const CONNECTION_RECORD_VERSION: u32 = 1;

fn current_record_version() -> u32 {
    CONNECTION_RECORD_VERSION
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionRecord {
    id: String,
    kind: String,
    base_url: String,
    display_name: String,
    #[serde(default = "current_record_version")]
    version: u32,
}

impl ConnectionRecord {
    fn from_connection(connection: &ProviderConnection) -> Self {
        Self {
            id: connection.id().as_str().to_string(),
            kind: connection.kind().as_str().to_string(),
            base_url: connection.base_url().as_str().to_string(),
            display_name: connection.display_name().as_str().to_string(),
            version: CONNECTION_RECORD_VERSION,
        }
    }

    fn to_connection(&self) -> Option<ProviderConnection> {
        let id = ConnectionId::parse(&self.id, InputField::ConnectionId).ok()?;
        let kind = ProviderKind::parse(&self.kind).ok()?;
        let base_url = BaseUrl::parse(&self.base_url).ok()?;
        let display_name = DisplayName::parse(Some(&self.display_name), &base_url).ok()?;
        ProviderConnection::restore(id, kind, base_url, display_name)
    }
}

/// Parses each stored element on its own so one corrupt entry never blocks the
/// others. Records written by a newer app version and invalid records are skipped.
fn valid_records(stored: Option<Value>) -> Vec<(ConnectionRecord, ProviderConnection)> {
    let elements = stored
        .and_then(|value| serde_json::from_value::<Vec<Value>>(value).ok())
        .unwrap_or_default();
    elements
        .into_iter()
        .filter_map(|element| serde_json::from_value::<ConnectionRecord>(element).ok())
        .filter(|record| record.version <= CONNECTION_RECORD_VERSION)
        .filter_map(|record| {
            let connection = record.to_connection()?;
            Some((record, connection))
        })
        .collect()
}

fn records_to_connections(stored: Option<Value>) -> Vec<ProviderConnection> {
    valid_records(stored)
        .into_iter()
        .map(|(_, connection)| connection)
        .collect()
}

fn upsert_record(
    mut records: Vec<ConnectionRecord>,
    record: ConnectionRecord,
) -> Vec<ConnectionRecord> {
    match records.iter_mut().find(|existing| existing.id == record.id) {
        Some(existing) => *existing = record,
        None => records.push(record),
    }
    records
}

fn remove_record(records: Vec<ConnectionRecord>, id: &ConnectionId) -> Vec<ConnectionRecord> {
    records
        .into_iter()
        .filter(|record| record.id != id.as_str())
        .collect()
}

/// Connection metadata (no secret) in `$APPDATA/work-tracking.json`.
pub struct PluginStoreConnectionRepository {
    app: AppHandle,
    write_lock: Mutex<()>,
}

impl PluginStoreConnectionRepository {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            write_lock: Mutex::new(()),
        }
    }

    fn read_stored(&self) -> Result<Option<Value>, WorkTrackingError> {
        let store = self
            .app
            .store(STORE_FILE)
            .map_err(|_| WorkTrackingError::StorageError)?;
        Ok(store.get(CONNECTIONS_KEY))
    }

    /// Read–modify–write under `write_lock`; Tauri runs async commands
    /// concurrently. The guard never crosses an `.await` (this is sync code).
    fn rewrite(
        &self,
        modify: impl FnOnce(Vec<ConnectionRecord>) -> Vec<ConnectionRecord>,
    ) -> Result<(), WorkTrackingError> {
        // The guarded data is `()`, so a poisoned lock is safe to reuse.
        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let store = self
            .app
            .store(STORE_FILE)
            .map_err(|_| WorkTrackingError::StorageError)?;
        let records = valid_records(store.get(CONNECTIONS_KEY))
            .into_iter()
            .map(|(record, _)| record)
            .collect();
        let updated =
            serde_json::to_value(modify(records)).map_err(|_| WorkTrackingError::StorageError)?;
        store.set(CONNECTIONS_KEY, updated);
        store.save().map_err(|_| WorkTrackingError::StorageError)
    }
}

impl ConnectionRepository for PluginStoreConnectionRepository {
    fn list(&self) -> Result<Vec<ProviderConnection>, WorkTrackingError> {
        Ok(records_to_connections(self.read_stored()?))
    }

    fn find(&self, id: &ConnectionId) -> Result<Option<ProviderConnection>, WorkTrackingError> {
        Ok(self
            .list()?
            .into_iter()
            .find(|connection| connection.id() == id))
    }

    fn save(&self, connection: &ProviderConnection) -> Result<(), WorkTrackingError> {
        let record = ConnectionRecord::from_connection(connection);
        self.rewrite(|records| upsert_record(records, record))
    }

    fn remove(&self, id: &ConnectionId) -> Result<(), WorkTrackingError> {
        self.rewrite(|records| remove_record(records, id))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn connection(host: &str) -> ProviderConnection {
        let base_url = BaseUrl::parse(&format!("https://{host}")).unwrap();
        let display_name = DisplayName::parse(None, &base_url).unwrap();
        ProviderConnection::new(ProviderKind::Teamwork, base_url, display_name)
    }

    #[test]
    fn reads_valid_records_and_skips_invalid_ones() {
        let stored = json!([
            { "id": "teamwork:acme.teamwork.com", "kind": "teamwork", "baseUrl": "https://acme.teamwork.com", "displayName": "Acme", "version": 1 },
            { "id": "teamwork:evil.example.com", "kind": "teamwork", "baseUrl": "https://acme.teamwork.com", "displayName": "Tampered", "version": 1 },
            { "id": "jira:acme.atlassian.net", "kind": "jira", "baseUrl": "https://acme.atlassian.net", "displayName": "Jira", "version": 1 },
            { "id": "teamwork:new.teamwork.com", "kind": "teamwork", "baseUrl": "https://new.teamwork.com", "displayName": "Newer", "version": 2 },
            { "id": "teamwork:old.teamwork.com", "kind": "teamwork", "baseUrl": "https://old.teamwork.com", "displayName": "Unversioned" },
            "not an object",
            { "id": 5 }
        ]);
        let connections = records_to_connections(Some(stored));
        let names: Vec<&str> = connections
            .iter()
            .map(|connection| connection.display_name().as_str())
            .collect();
        assert_eq!(names, vec!["Acme", "Unversioned"]);
    }

    #[test]
    fn missing_or_non_array_value_is_empty() {
        assert!(records_to_connections(None).is_empty());
        assert!(records_to_connections(Some(json!({ "id": "x" }))).is_empty());
        assert!(records_to_connections(Some(json!(null))).is_empty());
    }

    #[test]
    fn record_without_version_reads_as_current() {
        let record: ConnectionRecord = serde_json::from_value(json!({
            "id": "teamwork:acme.teamwork.com", "kind": "teamwork",
            "baseUrl": "https://acme.teamwork.com", "displayName": "Acme"
        }))
        .unwrap();
        assert_eq!(record.version, CONNECTION_RECORD_VERSION);
    }

    #[test]
    fn written_records_carry_version() {
        let record = ConnectionRecord::from_connection(&connection("acme.teamwork.com"));
        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(
            json,
            r#"{"id":"teamwork:acme.teamwork.com","kind":"teamwork","baseUrl":"https://acme.teamwork.com","displayName":"acme.teamwork.com","version":1}"#
        );
        assert_eq!(
            record.to_connection(),
            Some(connection("acme.teamwork.com"))
        );
    }

    #[test]
    fn upsert_replaces_by_id_or_appends() {
        let first = ConnectionRecord::from_connection(&connection("a.teamwork.com"));
        let second = ConnectionRecord::from_connection(&connection("b.teamwork.com"));
        let records = upsert_record(Vec::new(), first.clone());
        let records = upsert_record(records, second.clone());
        let mut renamed = first.clone();
        renamed.display_name = "Renamed".into();
        let records = upsert_record(records, renamed.clone());
        assert_eq!(records, vec![renamed, second]);
    }

    #[test]
    fn remove_filters_by_id() {
        let first = ConnectionRecord::from_connection(&connection("a.teamwork.com"));
        let second = ConnectionRecord::from_connection(&connection("b.teamwork.com"));
        let id = connection("a.teamwork.com").id().clone();
        let records = remove_record(vec![first, second.clone()], &id);
        assert_eq!(records, vec![second.clone()]);
        assert_eq!(remove_record(records, &id), vec![second]);
    }
}
