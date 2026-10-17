use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::workspace::domain::agent_run::{keep_recent_runs, AgentRun, AgentRunState};
use crate::workspace::domain::identifiers::{AgentRunId, ContextId};
use crate::workspace::domain::ports::{AgentRunRepository, StorageError};

/// A context's runs kept in memory, one entry per context, oldest first. The
/// lock is held for a whole read-modify-write so the caps always see the
/// latest list, like the redb write transaction.
#[derive(Default)]
pub struct InMemoryAgentRunRepository {
    runs_by_context: Mutex<BTreeMap<String, Vec<AgentRun>>>,
}

impl InMemoryAgentRunRepository {
    fn runs_by_context(&self) -> MutexGuard<'_, BTreeMap<String, Vec<AgentRun>>> {
        self.runs_by_context
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn mutate_run(
        &self,
        context_id: &ContextId,
        run_id: &AgentRunId,
        change: impl FnOnce(&mut AgentRun),
    ) {
        let mut runs_by_context = self.runs_by_context();
        let Some(runs) = runs_by_context.get_mut(context_id.as_str()) else {
            return;
        };
        if let Some(run) = runs.iter_mut().find(|run| &run.id == run_id) {
            change(run);
        }
    }
}

impl AgentRunRepository for InMemoryAgentRunRepository {
    fn start_run(&self, run: &AgentRun) -> Result<(), StorageError> {
        let mut runs_by_context = self.runs_by_context();
        let runs = runs_by_context
            .entry(run.context_id.as_str().to_string())
            .or_default();
        runs.push(run.clone());
        let kept = keep_recent_runs(std::mem::take(runs));
        *runs = kept;
        Ok(())
    }

    fn append_lines(
        &self,
        context_id: &ContextId,
        run_id: &AgentRunId,
        lines: &[String],
    ) -> Result<(), StorageError> {
        self.mutate_run(context_id, run_id, |run| {
            run.append_lines(lines.iter().cloned());
        });
        Ok(())
    }

    fn finish_run(
        &self,
        context_id: &ContextId,
        run_id: &AgentRunId,
        state: AgentRunState,
        exit_code: Option<i32>,
    ) -> Result<(), StorageError> {
        self.mutate_run(context_id, run_id, |run| run.end(state, exit_code));
        Ok(())
    }

    fn list_runs(&self, context_id: &ContextId) -> Result<Vec<AgentRun>, StorageError> {
        Ok(self
            .runs_by_context()
            .get(context_id.as_str())
            .cloned()
            .unwrap_or_default())
    }

    fn delete_runs(&self, context_id: &ContextId) -> Result<(), StorageError> {
        self.runs_by_context().remove(context_id.as_str());
        Ok(())
    }

    fn running_runs(&self) -> Result<Vec<AgentRun>, StorageError> {
        Ok(self
            .runs_by_context()
            .values()
            .flatten()
            .filter(|run| run.state.is_running())
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::infra::agent_run_repository_contract::agent_run_repository_contract_tests;

    fn with_repository(test: fn(&dyn AgentRunRepository)) {
        test(&InMemoryAgentRunRepository::default());
    }

    agent_run_repository_contract_tests!(with_repository);
}
