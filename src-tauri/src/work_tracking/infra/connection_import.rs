//! One-time import of connection metadata from the old plugin-store file
//! `work-tracking.json` into redb. On first run the records are copied into the
//! repository and the file is renamed to `work-tracking.json.imported`, so the
//! import never runs twice. API keys are untouched: they stay in the keychain.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::work_tracking::domain::base_url::BaseUrl;
use crate::work_tracking::domain::error::InputField;
use crate::work_tracking::domain::identifiers::ConnectionId;
use crate::work_tracking::domain::ports::ConnectionRepository;
use crate::work_tracking::domain::provider_connection::{DisplayName, ProviderConnection};
use crate::work_tracking::domain::provider_kind::ProviderKind;

/// The old plugin-store file name, in `$APPDATA`.
pub const LEGACY_STORE_FILE: &str = "work-tracking.json";
/// The suffix the file is renamed to once imported.
pub const IMPORTED_SUFFIX: &str = "work-tracking.json.imported";
const CONNECTIONS_KEY: &str = "connections";

/// One stored element of the old plugin-store `connections` array. `version` is
/// optional for records written before it existed.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyRecord {
    id: String,
    kind: String,
    base_url: String,
    display_name: String,
    #[serde(default)]
    version: Option<u32>,
}

const LATEST_LEGACY_VERSION: u32 = 1;

impl LegacyRecord {
    fn to_connection(&self) -> Option<ProviderConnection> {
        if self.version.unwrap_or(LATEST_LEGACY_VERSION) > LATEST_LEGACY_VERSION {
            return None;
        }
        let id = ConnectionId::parse(&self.id, InputField::ConnectionId).ok()?;
        let kind = ProviderKind::parse(&self.kind).ok()?;
        let base_url = BaseUrl::parse(&self.base_url).ok()?;
        let display_name = DisplayName::parse(Some(&self.display_name), &base_url).ok()?;
        ProviderConnection::restore(id, kind, base_url, display_name)
    }
}

/// Parses the connections out of the raw plugin-store JSON, skipping any
/// element that no longer validates so one bad record never blocks the others.
fn connections_from_store(contents: &str) -> Vec<ProviderConnection> {
    let Ok(store) = serde_json::from_str::<Value>(contents) else {
        return Vec::new();
    };
    let Some(elements) = store.get(CONNECTIONS_KEY).and_then(Value::as_array) else {
        return Vec::new();
    };
    elements
        .iter()
        .filter_map(|element| serde_json::from_value::<LegacyRecord>(element.clone()).ok())
        .filter_map(|record| record.to_connection())
        .collect()
}

/// Imports `directory/work-tracking.json` into `repository` when it is present
/// and not yet imported, then renames it to `work-tracking.json.imported`.
/// Does nothing when the file is absent. Returns the number imported.
pub fn import_legacy_connections(
    directory: &Path,
    repository: &dyn ConnectionRepository,
) -> std::io::Result<usize> {
    let source = directory.join(LEGACY_STORE_FILE);
    let contents = match std::fs::read_to_string(&source) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };

    let mut imported = 0;
    for connection in connections_from_store(&contents) {
        // A storage error on one connection should not abort the import of the
        // rest or leave the file un-renamed; it is logged and skipped.
        if repository.save(&connection).is_ok() {
            imported += 1;
        } else {
            eprintln!("[work_tracking] failed to import a connection into redb");
        }
    }

    let destination: PathBuf = directory.join(IMPORTED_SUFFIX);
    std::fs::rename(&source, &destination)?;
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_tracking::infra::in_memory_connection_repository::InMemoryConnectionRepository;

    fn store_json() -> &'static str {
        r#"{
            "connections": [
                { "id": "teamwork:acme.teamwork.com", "kind": "teamwork", "baseUrl": "https://acme.teamwork.com", "displayName": "Acme", "version": 1 },
                { "id": "teamwork:evil.example.com", "kind": "teamwork", "baseUrl": "https://acme.teamwork.com", "displayName": "Tampered", "version": 1 },
                { "id": "jira:acme.atlassian.net", "kind": "jira", "baseUrl": "https://acme.atlassian.net", "displayName": "Jira", "version": 1 },
                { "id": "teamwork:new.teamwork.com", "kind": "teamwork", "baseUrl": "https://new.teamwork.com", "displayName": "Newer", "version": 2 },
                { "id": "teamwork:old.teamwork.com", "kind": "teamwork", "baseUrl": "https://old.teamwork.com", "displayName": "Unversioned" }
            ]
        }"#
    }

    fn write_store(directory: &std::path::Path, contents: &str) {
        std::fs::write(directory.join(LEGACY_STORE_FILE), contents).unwrap();
    }

    #[test]
    fn imports_valid_records_and_skips_invalid_ones() {
        let directory = tempfile::tempdir().unwrap();
        write_store(directory.path(), store_json());
        let repository = InMemoryConnectionRepository::default();

        let imported = import_legacy_connections(directory.path(), &repository).unwrap();

        assert_eq!(imported, 2);
        let mut names: Vec<String> = repository
            .list()
            .unwrap()
            .iter()
            .map(|connection| connection.display_name().as_str().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["Acme".to_string(), "Unversioned".to_string()]);
    }

    #[test]
    fn renames_the_file_so_the_import_runs_once() {
        let directory = tempfile::tempdir().unwrap();
        write_store(directory.path(), store_json());
        let repository = InMemoryConnectionRepository::default();

        import_legacy_connections(directory.path(), &repository).unwrap();

        assert!(!directory.path().join(LEGACY_STORE_FILE).exists());
        assert!(directory.path().join(IMPORTED_SUFFIX).exists());

        // A second run finds no source file and imports nothing more.
        let imported_again = import_legacy_connections(directory.path(), &repository).unwrap();
        assert_eq!(imported_again, 0);
        assert_eq!(repository.list().unwrap().len(), 2);
    }

    #[test]
    fn a_missing_file_imports_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let repository = InMemoryConnectionRepository::default();

        let imported = import_legacy_connections(directory.path(), &repository).unwrap();

        assert_eq!(imported, 0);
        assert!(repository.list().unwrap().is_empty());
    }

    #[test]
    fn an_empty_or_malformed_store_imports_nothing_and_is_renamed() {
        let directory = tempfile::tempdir().unwrap();
        write_store(directory.path(), "not json at all");
        let repository = InMemoryConnectionRepository::default();

        let imported = import_legacy_connections(directory.path(), &repository).unwrap();

        assert_eq!(imported, 0);
        assert!(directory.path().join(IMPORTED_SUFFIX).exists());
    }
}
