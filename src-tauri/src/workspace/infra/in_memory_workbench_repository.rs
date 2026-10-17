use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::workspace::domain::ports::{StorageError, WorkbenchRepository};
use crate::workspace::domain::workbench::Workbench;

/// The one Workbench kept in memory. An unsaved repository reads as the empty
/// default, like a store with no record.
#[derive(Default)]
pub struct InMemoryWorkbenchRepository {
    workbench: Mutex<Option<Workbench>>,
}

impl InMemoryWorkbenchRepository {
    fn workbench(&self) -> MutexGuard<'_, Option<Workbench>> {
        // Every save replaces the whole record, so a poisoned slot is still consistent.
        self.workbench
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl WorkbenchRepository for InMemoryWorkbenchRepository {
    fn load(&self) -> Result<Workbench, StorageError> {
        Ok(self.workbench().clone().unwrap_or_default())
    }

    fn save(&self, workbench: &Workbench) -> Result<(), StorageError> {
        *self.workbench() = Some(workbench.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::infra::workbench_repository_contract::workbench_repository_contract_tests;

    fn with_repository(test: fn(&dyn WorkbenchRepository)) {
        test(&InMemoryWorkbenchRepository::default());
    }

    workbench_repository_contract_tests!(with_repository);
}
