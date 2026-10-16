//! The app's single redb database, `me.redb`. It is opened once at startup and
//! the handle is shared by every module that keeps tables in it; each module
//! creates its own tables.

use std::path::Path;
use std::sync::Arc;

use redb::{Database, DatabaseError};

pub const DATABASE_FILE: &str = "me.redb";

/// Opens `me.redb` in `app_data_folder`, creating the folder and the file when
/// missing.
pub fn open(app_data_folder: &Path) -> Result<Arc<Database>, DatabaseError> {
    std::fs::create_dir_all(app_data_folder)?;
    let database = Database::create(app_data_folder.join(DATABASE_FILE))?;
    Ok(Arc::new(database))
}

#[cfg(test)]
mod tests {
    use redb::{ReadableDatabase, TableDefinition};

    use super::*;

    const NESTED_FOLDER: &str = "missing/app-data";
    const OTHER_TABLE: TableDefinition<&str, u64> = TableDefinition::new("other");
    const KEY: &str = "key";
    const VALUE: u64 = 7;

    #[test]
    fn creates_the_folder_and_the_database_file_when_missing() {
        let directory = tempfile::tempdir().unwrap();
        let app_data_folder = directory.path().join(NESTED_FOLDER);

        open(&app_data_folder).unwrap();

        assert!(app_data_folder.join(DATABASE_FILE).is_file());
    }

    #[test]
    fn reopening_reads_what_was_written() {
        let directory = tempfile::tempdir().unwrap();
        let database = open(directory.path()).unwrap();
        let transaction = database.begin_write().unwrap();
        transaction
            .open_table(OTHER_TABLE)
            .unwrap()
            .insert(KEY, VALUE)
            .unwrap();
        transaction.commit().unwrap();
        drop(database);

        let reopened = open(directory.path()).unwrap();

        let transaction = reopened.begin_read().unwrap();
        let table = transaction.open_table(OTHER_TABLE).unwrap();
        assert_eq!(table.get(KEY).unwrap().unwrap().value(), VALUE);
    }
}
