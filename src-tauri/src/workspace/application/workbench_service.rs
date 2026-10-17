//! The WorkbenchService: the arrange use cases over the one Workbench. Each
//! use case loads the record, applies a pure mutator, reconciles the result
//! against the live project list, saves it, emits `WorkbenchChanged` and
//! returns the reconciled arrangement.
//!
//! `get` is the exception: it reconciles in memory and returns the result
//! without saving, so reading the arrangement stays a read. The next mutation
//! persists whatever reconcile folded in (a new project, a dropped one, a
//! cleared last-open tag), so nothing is lost by not saving on read.

use std::sync::Arc;

use crate::workspace::domain::ports::{
    ProjectRepository, StorageError, WorkbenchEvents, WorkbenchRepository,
};
use crate::workspace::domain::project_tag::ProjectTag;
use crate::workspace::domain::workbench::{Placement, Workbench};

pub struct WorkbenchService {
    workbench_repository: Arc<dyn WorkbenchRepository>,
    project_repository: Arc<dyn ProjectRepository>,
    events: Arc<dyn WorkbenchEvents>,
}

impl WorkbenchService {
    pub fn new(
        workbench_repository: Arc<dyn WorkbenchRepository>,
        project_repository: Arc<dyn ProjectRepository>,
        events: Arc<dyn WorkbenchEvents>,
    ) -> Self {
        Self {
            workbench_repository,
            project_repository,
            events,
        }
    }

    /// The reconciled arrangement, read-only: loads the record and folds it
    /// against the live project list in memory, without saving.
    pub fn get(&self) -> Result<Workbench, StorageError> {
        let workbench = self.workbench_repository.load()?;
        Ok(workbench.reconcile(&self.live_project_tags()?))
    }

    /// Pins, unpins (→ Shown), hides or shows (→ Shown at the end) a project.
    pub fn set_placement(
        &self,
        project_tag: &ProjectTag,
        placement: Placement,
    ) -> Result<Workbench, StorageError> {
        self.change(|workbench| workbench.set_placement(project_tag, placement))
    }

    /// Reorders the entries of one placement group to match `ordered_tags`.
    pub fn reorder_within(
        &self,
        placement: Placement,
        ordered_tags: &[ProjectTag],
    ) -> Result<Workbench, StorageError> {
        self.change(|workbench| workbench.reorder_within(placement, ordered_tags))
    }

    /// Sets the project the app reopens on; `None` clears it.
    pub fn set_last_project(
        &self,
        project_tag: Option<ProjectTag>,
    ) -> Result<Workbench, StorageError> {
        self.change(|workbench| workbench.set_last_project(project_tag))
    }

    /// Loads the record and reconciles it against the live project list first,
    /// so every live project has an entry the mutator can act on (a brand-new
    /// project that only exists in the project list gets its `Shown` entry
    /// before, say, being pinned). Then applies `mutate`, reconciles again to
    /// fold the result back into shape, saves it, emits the change and returns
    /// the reconciled arrangement.
    fn change(
        &self,
        mutate: impl FnOnce(Workbench) -> Workbench,
    ) -> Result<Workbench, StorageError> {
        let live_project_tags = self.live_project_tags()?;
        let reconciled = self
            .workbench_repository
            .load()?
            .reconcile(&live_project_tags);
        let updated = mutate(reconciled).reconcile(&live_project_tags);
        self.workbench_repository.save(&updated)?;
        self.events.workbench_changed();
        Ok(updated)
    }

    /// The tags of the readable projects, in the repository's tag-alphabetical
    /// order. Unreadable records are excluded, which is exactly what reconcile
    /// treats as "gone".
    fn live_project_tags(&self) -> Result<Vec<ProjectTag>, StorageError> {
        Ok(self
            .project_repository
            .list()?
            .projects
            .iter()
            .map(|project| project.tag().clone())
            .collect())
    }
}

#[cfg(test)]
#[path = "workbench_service_tests.rs"]
mod tests;
