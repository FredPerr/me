use std::fmt::Display;
use std::sync::Arc;

use redb::{Database, ReadableDatabase, TableDefinition};

use crate::settings::domain::app_settings::AppSettings;
use crate::settings::domain::ports::{SettingsStore, SettingsStoreUnavailable};

/// One record under a fixed key, holding the settings as MessagePack with field
/// names so added fields still read from older records.
const SETTINGS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("settings");

/// The key of the single settings record.
const SETTINGS_KEY: &str = "app";

fn unavailable(error: impl Display) -> SettingsStoreUnavailable {
    eprintln!("[settings] settings storage failed: {error}");
    SettingsStoreUnavailable
}

/// An unreadable record reads as the defaults, so it is replaced on the next save.
fn decode(bytes: &[u8]) -> AppSettings {
    match rmp_serde::from_slice(bytes) {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("[settings] settings record unreadable: {error}");
            AppSettings::default()
        }
    }
}

/// The app settings in the shared redb database.
pub struct RedbSettingsStore {
    database: Arc<Database>,
}

impl RedbSettingsStore {
    /// Uses the shared database, creating its `settings` table when missing.
    pub fn new(database: Arc<Database>) -> Result<Self, SettingsStoreUnavailable> {
        let transaction = database.begin_write().map_err(unavailable)?;
        transaction
            .open_table(SETTINGS_TABLE)
            .map_err(unavailable)?;
        transaction.commit().map_err(unavailable)?;
        Ok(Self { database })
    }
}

impl SettingsStore for RedbSettingsStore {
    fn load(&self) -> Result<AppSettings, SettingsStoreUnavailable> {
        let transaction = self.database.begin_read().map_err(unavailable)?;
        let table = transaction
            .open_table(SETTINGS_TABLE)
            .map_err(unavailable)?;
        let record = table.get(SETTINGS_KEY).map_err(unavailable)?;
        Ok(record
            .map(|record| decode(record.value()))
            .unwrap_or_default())
    }

    fn save(&self, settings: &AppSettings) -> Result<(), SettingsStoreUnavailable> {
        let record = rmp_serde::to_vec_named(settings).map_err(unavailable)?;
        let transaction = self.database.begin_write().map_err(unavailable)?;
        {
            let mut table = transaction
                .open_table(SETTINGS_TABLE)
                .map_err(unavailable)?;
            table
                .insert(SETTINGS_KEY, record.as_slice())
                .map_err(unavailable)?;
        }
        transaction.commit().map_err(unavailable)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::settings::domain::ai_tool::AiTool;
    use crate::settings::infra::settings_store_contract::settings_store_contract_tests;
    use crate::storage;

    const CORRUPT_BYTES: &[u8] = b"not settings";

    fn store_in(directory: &TempDir) -> RedbSettingsStore {
        RedbSettingsStore::new(storage::open(directory.path()).unwrap()).unwrap()
    }

    fn with_store(test: fn(&dyn SettingsStore)) {
        let directory = tempfile::tempdir().unwrap();
        test(&store_in(&directory));
    }

    settings_store_contract_tests!(with_store);

    fn write_raw_record(store: &RedbSettingsStore, bytes: &[u8]) {
        let transaction = store.database.begin_write().unwrap();
        {
            let mut table = transaction.open_table(SETTINGS_TABLE).unwrap();
            table.insert(SETTINGS_KEY, bytes).unwrap();
        }
        transaction.commit().unwrap();
    }

    #[test]
    fn loads_the_defaults_on_a_fresh_database() {
        let directory = tempfile::tempdir().unwrap();

        assert_eq!(store_in(&directory).load().unwrap(), AppSettings::default());
    }

    #[test]
    fn reopening_the_database_keeps_the_settings() {
        let directory = tempfile::tempdir().unwrap();
        let settings = AppSettings {
            assign_me_to_tasks: false,
            default_ai_tool: AiTool::Kiro,
        };
        store_in(&directory).save(&settings).unwrap();

        let reopened = store_in(&directory);

        assert_eq!(reopened.load().unwrap(), settings);
    }

    #[test]
    fn reopening_the_database_keeps_the_default_ai_tool() {
        let directory = tempfile::tempdir().unwrap();
        let settings = AppSettings {
            assign_me_to_tasks: true,
            default_ai_tool: AiTool::Kiro,
        };
        store_in(&directory).save(&settings).unwrap();

        let reopened = store_in(&directory);

        assert_eq!(reopened.load().unwrap().default_ai_tool, AiTool::Kiro);
    }

    #[test]
    fn a_record_written_before_the_default_ai_tool_existed_loads_as_kiro() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        let record = rmp_serde::to_vec_named(&json!({ "assignMeToTasks": false })).unwrap();

        write_raw_record(&store, &record);

        assert_eq!(store.load().unwrap().default_ai_tool, AiTool::Kiro);
    }

    #[test]
    fn stores_the_settings_with_field_names() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);
        let record = rmp_serde::to_vec_named(&json!({ "assignMeToTasks": false })).unwrap();

        write_raw_record(&store, &record);

        assert_eq!(
            store.load().unwrap(),
            AppSettings {
                assign_me_to_tasks: false,
                default_ai_tool: AiTool::Kiro,
            }
        );
    }

    #[test]
    fn an_unreadable_record_reads_as_the_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let store = store_in(&directory);

        write_raw_record(&store, CORRUPT_BYTES);

        assert_eq!(store.load().unwrap(), AppSettings::default());
    }
}
