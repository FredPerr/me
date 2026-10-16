use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::workspace::domain::ports::{
    ProjectChange, ProjectListing, ProjectRepository, StorageError, UpdateError,
};
use crate::workspace::domain::project::Project;
use crate::workspace::domain::project_tag::ProjectTag;

/// Projects kept in memory, ordered by tag. The lock is held for a whole
/// `update`, like the redb write transaction.
#[derive(Default)]
pub struct InMemoryProjectRepository {
    projects: Mutex<BTreeMap<String, Project>>,
}

impl InMemoryProjectRepository {
    fn projects(&self) -> MutexGuard<'_, BTreeMap<String, Project>> {
        // Every write replaces whole Projects, so a poisoned map is still consistent.
        self.projects.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ProjectRepository for InMemoryProjectRepository {
    fn list(&self) -> Result<ProjectListing, StorageError> {
        Ok(ProjectListing {
            projects: self.projects().values().cloned().collect(),
            unreadable: vec![],
        })
    }

    fn find(&self, tag: &ProjectTag) -> Result<Option<Project>, StorageError> {
        Ok(self.projects().get(tag.as_str()).cloned())
    }

    fn save(&self, project: &Project) -> Result<(), StorageError> {
        self.projects()
            .insert(project.tag().as_str().to_string(), project.clone());
        Ok(())
    }

    fn delete(&self, tag: &ProjectTag) -> Result<(), StorageError> {
        self.projects().remove(tag.as_str());
        Ok(())
    }

    fn update(&self, tag: &ProjectTag, change: ProjectChange<'_>) -> Result<Project, UpdateError> {
        let mut projects = self.projects();
        let current = projects
            .get(tag.as_str())
            .cloned()
            .ok_or_else(|| UpdateError::NotFound { tag: tag.clone() })?;
        let updated = change(current).map_err(UpdateError::Rejected)?;
        projects.insert(tag.as_str().to_string(), updated.clone());
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::infra::project_repository_contract::project_repository_contract_tests;

    fn with_repository(test: fn(&dyn ProjectRepository)) {
        test(&InMemoryProjectRepository::default());
    }

    project_repository_contract_tests!(with_repository);
}
