use std::fmt::Display;
use std::sync::Arc;

use redb::{Database, ReadableDatabase, TableDefinition};

use crate::workspace::domain::ports::{StorageError, WorkbenchRepository};
use crate::workspace::domain::project_tag::ProjectTag;
use crate::workspace::domain::workbench::Workbench;

/// The one Workbench record, holding it as MessagePack with field names so
/// added fields still read from older records.
const WORKBENCH_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("workbench");

/// The single key the one record lives under; there is one Workbench per app.
const WORKBENCH_KEY: &str = "workbench";

fn unavailable(error: impl Display) -> StorageError {
    StorageError::Unavailable {
        reason: error.to_string(),
    }
}

fn encode(workbench: &Workbench) -> Result<Vec<u8>, StorageError> {
    rmp_serde::to_vec_named(workbench).map_err(|error| StorageError::Unencodable {
        tag: ProjectTag::parse(WORKBENCH_KEY).expect("the workbench key is never blank"),
        reason: error.to_string(),
    })
}

/// An unreadable record reads as the empty default, so the bar never breaks.
fn decode(bytes: &[u8]) -> Workbench {
    match rmp_serde::from_slice(bytes) {
        Ok(workbench) => workbench,
        Err(error) => {
            eprintln!("[workspace] workbench record unreadable: {error}");
            Workbench::default()
        }
    }
}

/// The Workbench in the shared redb database.
pub struct RedbWorkbenchRepository {
    database: Arc<Database>,
}

impl RedbWorkbenchRepository {
    /// Uses the shared database, creating its `workbench` table when missing.
    pub fn new(database: Arc<Database>) -> Result<Self, StorageError> {
        let transaction = database.begin_write().map_err(unavailable)?;
        transaction
            .open_table(WORKBENCH_TABLE)
            .map_err(unavailable)?;
        transaction.commit().map_err(unavailable)?;
        Ok(Self { database })
    }

    fn write(
        &self,
        apply: impl FnOnce(&mut redb::Table<&str, &[u8]>) -> Result<(), StorageError>,
    ) -> Result<(), StorageError> {
        let transaction = self.database.begin_write().map_err(unavailable)?;
        {
            let mut table = transaction
                .open_table(WORKBENCH_TABLE)
                .map_err(unavailable)?;
            apply(&mut table)?;
        }
        transaction.commit().map_err(unavailable)
    }
}

impl WorkbenchRepository for RedbWorkbenchRepository {
    fn load(&self) -> Result<Workbench, StorageError> {
        let transaction = self.database.begin_read().map_err(unavailable)?;
        let table = transaction
            .open_table(WORKBENCH_TABLE)
            .map_err(unavailable)?;
        let Some(record) = table.get(WORKBENCH_KEY).map_err(unavailable)? else {
            return Ok(Workbench::default());
        };
        Ok(decode(record.value()))
    }

    fn save(&self, workbench: &Workbench) -> Result<(), StorageError> {
        let record = encode(workbench)?;
        self.write(|table| {
            table
                .insert(WORKBENCH_KEY, record.as_slice())
                .map_err(unavailable)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use tempfile::TempDir;

    use super::*;
    use crate::storage;
    use crate::workspace::domain::workbench::{Placement, WorkbenchEntry};
    use crate::workspace::infra::workbench_repository_contract::workbench_repository_contract_tests;

    const EXISTING_TAG: &str = "existing";

    struct TemporaryDatabase {
        directory: TempDir,
        repository: RedbWorkbenchRepository,
    }

    impl TemporaryDatabase {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let repository = repository_in(&directory);
            Self {
                directory,
                repository,
            }
        }

        fn reopen(self) -> Self {
            let Self {
                directory,
                repository,
            } = self;
            drop(repository);
            let repository = repository_in(&directory);
            Self {
                directory,
                repository,
            }
        }

        fn write_raw_record(&self, bytes: &[u8]) {
            self.repository
                .write(|table| {
                    table.insert(WORKBENCH_KEY, bytes).map_err(unavailable)?;
                    Ok(())
                })
                .unwrap();
        }
    }

    fn repository_in(directory: &TempDir) -> RedbWorkbenchRepository {
        RedbWorkbenchRepository::new(storage::open(directory.path()).unwrap()).unwrap()
    }

    fn with_repository(test: fn(&dyn WorkbenchRepository)) {
        let database = TemporaryDatabase::new();
        test(&database.repository);
    }

    fn workbench_with_entry() -> Workbench {
        Workbench {
            entries: vec![WorkbenchEntry {
                project_tag: ProjectTag::parse(EXISTING_TAG).unwrap(),
                placement: Placement::Pinned,
            }],
            last_project_tag: Some(ProjectTag::parse(EXISTING_TAG).unwrap()),
        }
    }

    workbench_repository_contract_tests!(with_repository);

    #[test]
    fn reopening_the_database_keeps_the_workbench() {
        let database = TemporaryDatabase::new();
        let workbench = workbench_with_entry();
        database.repository.save(&workbench).unwrap();

        let reopened = database.reopen();

        assert_eq!(reopened.repository.load().unwrap(), workbench);
    }

    #[test]
    fn an_unreadable_record_reads_as_the_empty_default() {
        let database = TemporaryDatabase::new();
        database.write_raw_record(b"not a workbench");

        let loaded = database.repository.load().unwrap();

        assert_eq!(loaded, Workbench::default());
    }

    #[test]
    fn reads_a_record_written_before_the_last_project_field_existed() {
        let database = TemporaryDatabase::new();
        let workbench = workbench_with_entry();
        let mut older_record = serde_json::to_value(&workbench).unwrap();
        older_record
            .as_object_mut()
            .unwrap()
            .remove("lastProjectTag");
        let older_bytes = rmp_serde::to_vec_named::<Value>(&older_record).unwrap();
        database.write_raw_record(&older_bytes);

        let loaded = database.repository.load().unwrap();

        assert_eq!(loaded.entries, workbench.entries);
        assert_eq!(loaded.last_project_tag, None);
    }
}
