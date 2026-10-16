use std::fmt::Display;
use std::sync::Arc;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

use crate::workspace::domain::ports::{
    ProjectChange, ProjectListing, ProjectRepository, StorageError, UnreadableProject, UpdateError,
};
use crate::workspace::domain::project::Project;
use crate::workspace::domain::project_tag::ProjectTag;

/// One record per Project, keyed by tag, holding the Project as MessagePack
/// with field names so added fields still read from older records.
const PROJECTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("projects");

fn unavailable(error: impl Display) -> StorageError {
    StorageError::Unavailable {
        reason: error.to_string(),
    }
}

fn encode(project: &Project) -> Result<Vec<u8>, StorageError> {
    rmp_serde::to_vec_named(project).map_err(|error| StorageError::Unencodable {
        tag: project.tag().clone(),
        reason: error.to_string(),
    })
}

fn decode(tag: &str, bytes: &[u8]) -> Result<Project, UnreadableProject> {
    rmp_serde::from_slice(bytes).map_err(|error| UnreadableProject {
        tag: tag.to_string(),
        reason: error.to_string(),
    })
}

/// Projects in the shared redb database. redb allows one write transaction at
/// a time, so every write, and every `update` as a whole, is serialized.
pub struct RedbProjectRepository {
    database: Arc<Database>,
}

impl RedbProjectRepository {
    /// Uses the shared database, creating its `projects` table when missing.
    pub fn new(database: Arc<Database>) -> Result<Self, StorageError> {
        let transaction = database.begin_write().map_err(unavailable)?;
        transaction
            .open_table(PROJECTS_TABLE)
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
                .open_table(PROJECTS_TABLE)
                .map_err(unavailable)?;
            apply(&mut table)?;
        }
        transaction.commit().map_err(unavailable)
    }
}

impl ProjectRepository for RedbProjectRepository {
    fn list(&self) -> Result<ProjectListing, StorageError> {
        let transaction = self.database.begin_read().map_err(unavailable)?;
        let table = transaction
            .open_table(PROJECTS_TABLE)
            .map_err(unavailable)?;
        let mut listing = ProjectListing::default();
        for entry in table.iter().map_err(unavailable)? {
            let (key, value) = entry.map_err(unavailable)?;
            match decode(key.value(), value.value()) {
                Ok(project) => listing.projects.push(project),
                Err(unreadable) => listing.unreadable.push(unreadable),
            }
        }
        Ok(listing)
    }

    fn find(&self, tag: &ProjectTag) -> Result<Option<Project>, StorageError> {
        let transaction = self.database.begin_read().map_err(unavailable)?;
        let table = transaction
            .open_table(PROJECTS_TABLE)
            .map_err(unavailable)?;
        let Some(record) = table.get(tag.as_str()).map_err(unavailable)? else {
            return Ok(None);
        };
        decode(tag.as_str(), record.value())
            .map(Some)
            .map_err(StorageError::UnreadableRecord)
    }

    fn save(&self, project: &Project) -> Result<(), StorageError> {
        let record = encode(project)?;
        self.write(|table| {
            table
                .insert(project.tag().as_str(), record.as_slice())
                .map_err(unavailable)?;
            Ok(())
        })
    }

    fn delete(&self, tag: &ProjectTag) -> Result<(), StorageError> {
        self.write(|table| {
            table.remove(tag.as_str()).map_err(unavailable)?;
            Ok(())
        })
    }

    /// Returning before `commit` drops the transaction, which aborts it.
    fn update(&self, tag: &ProjectTag, change: ProjectChange<'_>) -> Result<Project, UpdateError> {
        let transaction = self.database.begin_write().map_err(unavailable)?;
        let updated = {
            let mut table = transaction
                .open_table(PROJECTS_TABLE)
                .map_err(unavailable)?;
            let current = match table.get(tag.as_str()).map_err(unavailable)? {
                Some(record) => {
                    decode(tag.as_str(), record.value()).map_err(StorageError::UnreadableRecord)?
                }
                None => return Err(UpdateError::NotFound { tag: tag.clone() }),
            };
            let updated = change(current).map_err(UpdateError::Rejected)?;
            table
                .insert(tag.as_str(), encode(&updated)?.as_slice())
                .map_err(unavailable)?;
            updated
        };
        transaction.commit().map_err(unavailable)?;
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use tempfile::TempDir;

    use super::*;
    use crate::storage;
    use crate::workspace::domain::test_support::{project_with, PROJECT_TAG};
    use crate::workspace::infra::project_repository_contract::{
        project_repository_contract_tests, project_tagged, project_with_contexts,
    };

    const READABLE_TAG: &str = "readable";
    const CORRUPT_TAG: &str = "corrupt";
    const CORRUPT_BYTES: &[u8] = b"not a project";
    const OTHER_TABLE: TableDefinition<&str, &str> = TableDefinition::new("other");
    const OTHER_KEY: &str = "account";
    const OTHER_VALUE: &str = "summary";
    const NEWER_OPTIONAL_FIELDS: [&str; 3] = ["icon", "remoteProjectLinks", "branchNaming"];

    struct TemporaryDatabase {
        directory: TempDir,
        repository: RedbProjectRepository,
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

        fn write_raw_record(&self, tag: &str, bytes: &[u8]) {
            self.repository
                .write(|table| {
                    table.insert(tag, bytes).map_err(unavailable)?;
                    Ok(())
                })
                .unwrap();
        }
    }

    fn repository_in(directory: &TempDir) -> RedbProjectRepository {
        RedbProjectRepository::new(storage::open(directory.path()).unwrap()).unwrap()
    }

    fn with_repository(test: fn(&dyn ProjectRepository)) {
        let database = TemporaryDatabase::new();
        test(&database.repository);
    }

    project_repository_contract_tests!(with_repository);

    #[test]
    fn reopening_the_database_keeps_the_projects() {
        let database = TemporaryDatabase::new();
        let project = project_with_contexts();
        database.repository.save(&project).unwrap();

        let reopened = database.reopen();

        assert_eq!(
            reopened.repository.find(project.tag()).unwrap(),
            Some(project)
        );
    }

    #[test]
    fn two_repositories_on_one_database_share_the_projects() {
        let directory = tempfile::tempdir().unwrap();
        let database = storage::open(directory.path()).unwrap();
        let writer = RedbProjectRepository::new(Arc::clone(&database)).unwrap();
        let reader = RedbProjectRepository::new(database).unwrap();
        let project = project_with_contexts();

        writer.save(&project).unwrap();

        assert_eq!(reader.find(project.tag()).unwrap(), Some(project));
    }

    #[test]
    fn another_table_in_the_same_database_leaves_the_projects_alone() {
        let directory = tempfile::tempdir().unwrap();
        let database = storage::open(directory.path()).unwrap();
        let repository = RedbProjectRepository::new(Arc::clone(&database)).unwrap();
        let project = project_with_contexts();
        repository.save(&project).unwrap();

        let transaction = database.begin_write().unwrap();
        transaction
            .open_table(OTHER_TABLE)
            .unwrap()
            .insert(OTHER_KEY, OTHER_VALUE)
            .unwrap();
        transaction.commit().unwrap();

        assert_eq!(repository.find(project.tag()).unwrap(), Some(project));
        let transaction = database.begin_read().unwrap();
        let table = transaction.open_table(OTHER_TABLE).unwrap();
        assert_eq!(table.get(OTHER_KEY).unwrap().unwrap().value(), OTHER_VALUE);
    }

    #[test]
    fn list_reports_a_corrupt_record_and_still_lists_the_others() {
        let database = TemporaryDatabase::new();
        let readable = project_tagged(READABLE_TAG);
        database.repository.save(&readable).unwrap();
        database.write_raw_record(CORRUPT_TAG, CORRUPT_BYTES);

        let listing = database.repository.list().unwrap();

        assert_eq!(listing.projects, vec![readable]);
        assert_eq!(listing.unreadable.len(), 1);
        assert_eq!(listing.unreadable[0].tag, CORRUPT_TAG);
    }

    #[test]
    fn find_reports_a_corrupt_record() {
        let database = TemporaryDatabase::new();
        database.write_raw_record(CORRUPT_TAG, CORRUPT_BYTES);

        let result = database
            .repository
            .find(&ProjectTag::parse(CORRUPT_TAG).unwrap());

        assert!(matches!(
            result,
            Err(StorageError::UnreadableRecord(UnreadableProject { tag, .. })) if tag == CORRUPT_TAG
        ));
    }

    #[test]
    fn reads_a_record_written_before_newer_optional_fields_existed() {
        let database = TemporaryDatabase::new();
        let project = project_with(vec![]);
        let mut older_record = serde_json::to_value(&project).unwrap();
        let fields = older_record.as_object_mut().unwrap();
        for field in NEWER_OPTIONAL_FIELDS {
            fields.remove(field);
        }
        let older_bytes = rmp_serde::to_vec_named::<Value>(&older_record).unwrap();
        database.write_raw_record(PROJECT_TAG, &older_bytes);

        let found = database.repository.find(project.tag()).unwrap();

        assert_eq!(found, Some(project));
    }
}
