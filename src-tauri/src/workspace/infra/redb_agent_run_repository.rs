use std::fmt::Display;
use std::sync::Arc;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

use crate::workspace::domain::agent_run::{keep_recent_runs, AgentRun, AgentRunState};
use crate::workspace::domain::identifiers::{AgentRunId, ContextId};
use crate::workspace::domain::ports::{AgentRunRepository, StorageError};

/// One record per context, keyed by context id, holding that context's runs as
/// a MessagePack-with-field-names list so added fields still read from older
/// records. A per-context list makes the last-ten cap a single record rewrite.
const AGENT_RUNS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("agent_runs");

fn unavailable(error: impl Display) -> StorageError {
    StorageError::Unavailable {
        reason: error.to_string(),
    }
}

fn encode(runs: &[AgentRun]) -> Result<Vec<u8>, StorageError> {
    rmp_serde::to_vec_named(runs).map_err(|error| StorageError::Unavailable {
        reason: error.to_string(),
    })
}

/// An unreadable record reads as no runs, so a corrupt list never blocks a
/// context from recording new runs.
fn decode(context_id: &str, bytes: &[u8]) -> Vec<AgentRun> {
    match rmp_serde::from_slice(bytes) {
        Ok(runs) => runs,
        Err(error) => {
            eprintln!("[workspace] agent-run record '{context_id}' unreadable: {error}");
            Vec::new()
        }
    }
}

/// Agent-run transcripts in the shared redb database. redb allows one write
/// transaction at a time, so every read-modify-write of a context's run list
/// is serialized and the caps always see the latest list.
pub struct RedbAgentRunRepository {
    database: Arc<Database>,
}

impl RedbAgentRunRepository {
    /// Uses the shared database, creating its `agent_runs` table when missing.
    pub fn new(database: Arc<Database>) -> Result<Self, StorageError> {
        let transaction = database.begin_write().map_err(unavailable)?;
        transaction
            .open_table(AGENT_RUNS_TABLE)
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
                .open_table(AGENT_RUNS_TABLE)
                .map_err(unavailable)?;
            apply(&mut table)?;
        }
        transaction.commit().map_err(unavailable)
    }

    /// Reads a context's runs, applies `change`, and writes the result back,
    /// all in one write transaction so concurrent changes never lose updates.
    fn update_runs(
        &self,
        context_id: &ContextId,
        change: impl FnOnce(&mut Vec<AgentRun>),
    ) -> Result<(), StorageError> {
        self.write(|table| {
            let mut runs = match table.get(context_id.as_str()).map_err(unavailable)? {
                Some(record) => decode(context_id.as_str(), record.value()),
                None => Vec::new(),
            };
            change(&mut runs);
            table
                .insert(context_id.as_str(), encode(&runs)?.as_slice())
                .map_err(unavailable)?;
            Ok(())
        })
    }
}

impl AgentRunRepository for RedbAgentRunRepository {
    fn start_run(&self, run: &AgentRun) -> Result<(), StorageError> {
        self.update_runs(&run.context_id, |runs| {
            runs.push(run.clone());
            *runs = keep_recent_runs(std::mem::take(runs));
        })
    }

    fn append_lines(
        &self,
        context_id: &ContextId,
        run_id: &AgentRunId,
        lines: &[String],
    ) -> Result<(), StorageError> {
        self.update_runs(context_id, |runs| {
            if let Some(run) = runs.iter_mut().find(|run| &run.id == run_id) {
                run.append_lines(lines.iter().cloned());
            }
        })
    }

    fn finish_run(
        &self,
        context_id: &ContextId,
        run_id: &AgentRunId,
        state: AgentRunState,
        exit_code: Option<i32>,
    ) -> Result<(), StorageError> {
        self.update_runs(context_id, |runs| {
            if let Some(run) = runs.iter_mut().find(|run| &run.id == run_id) {
                run.end(state, exit_code);
            }
        })
    }

    fn list_runs(&self, context_id: &ContextId) -> Result<Vec<AgentRun>, StorageError> {
        let transaction = self.database.begin_read().map_err(unavailable)?;
        let table = transaction
            .open_table(AGENT_RUNS_TABLE)
            .map_err(unavailable)?;
        let Some(record) = table.get(context_id.as_str()).map_err(unavailable)? else {
            return Ok(Vec::new());
        };
        Ok(decode(context_id.as_str(), record.value()))
    }

    fn delete_runs(&self, context_id: &ContextId) -> Result<(), StorageError> {
        self.write(|table| {
            table.remove(context_id.as_str()).map_err(unavailable)?;
            Ok(())
        })
    }

    fn running_runs(&self) -> Result<Vec<AgentRun>, StorageError> {
        let transaction = self.database.begin_read().map_err(unavailable)?;
        let table = transaction
            .open_table(AGENT_RUNS_TABLE)
            .map_err(unavailable)?;
        let mut running = Vec::new();
        for entry in table.iter().map_err(unavailable)? {
            let (key, value) = entry.map_err(unavailable)?;
            for run in decode(key.value(), value.value()) {
                if run.state.is_running() {
                    running.push(run);
                }
            }
        }
        Ok(running)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::storage;
    use crate::workspace::infra::agent_run_repository_contract::agent_run_repository_contract_tests;

    const CONTEXT: &str = "context-1";
    const PROMPT: &str = "Build the login page";
    const STARTED_AT: &str = "2024-01-15T10:00:00.000Z";
    const CORRUPT_BYTES: &[u8] = b"not a run list";

    fn repository_in(directory: &TempDir) -> RedbAgentRunRepository {
        RedbAgentRunRepository::new(storage::open(directory.path()).unwrap()).unwrap()
    }

    fn with_repository(test: fn(&dyn AgentRunRepository)) {
        let directory = tempfile::tempdir().unwrap();
        test(&repository_in(&directory));
    }

    agent_run_repository_contract_tests!(with_repository);

    fn a_run() -> AgentRun {
        AgentRun::started(
            AgentRunId::generate(),
            ContextId::parse(CONTEXT).unwrap(),
            PROMPT.to_string(),
            STARTED_AT.to_string(),
        )
    }

    fn write_raw_record(repository: &RedbAgentRunRepository, context_id: &str, bytes: &[u8]) {
        repository
            .write(|table| {
                table.insert(context_id, bytes).map_err(unavailable)?;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn reopening_the_database_keeps_the_runs() {
        let directory = tempfile::tempdir().unwrap();
        let run = a_run();
        repository_in(&directory).start_run(&run).unwrap();

        let reopened = repository_in(&directory);

        assert_eq!(
            reopened
                .list_runs(&ContextId::parse(CONTEXT).unwrap())
                .unwrap(),
            vec![run]
        );
    }

    #[test]
    fn an_unreadable_record_reads_as_no_runs() {
        let directory = tempfile::tempdir().unwrap();
        let repository = repository_in(&directory);

        write_raw_record(&repository, CONTEXT, CORRUPT_BYTES);

        let context = ContextId::parse(CONTEXT).unwrap();
        assert_eq!(repository.list_runs(&context).unwrap(), vec![]);
        assert_eq!(repository.running_runs().unwrap(), vec![]);
    }

    #[test]
    fn starting_a_run_replaces_an_unreadable_record() {
        let directory = tempfile::tempdir().unwrap();
        let repository = repository_in(&directory);
        write_raw_record(&repository, CONTEXT, CORRUPT_BYTES);
        let run = a_run();

        repository.start_run(&run).unwrap();

        assert_eq!(
            repository
                .list_runs(&ContextId::parse(CONTEXT).unwrap())
                .unwrap(),
            vec![run]
        );
    }
}
