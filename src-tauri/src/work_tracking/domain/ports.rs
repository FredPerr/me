use async_trait::async_trait;

use super::api_key::ApiKey;
use super::error::WorkTrackingError;
use super::identifiers::{ConnectionId, WorkItemGroupId, WorkProjectId};
use super::page::{Page, PageRequest};
use super::provider_connection::ProviderConnection;
use super::work_item::WorkItem;
use super::work_item_group::WorkItemGroup;
use super::work_project::WorkProject;

/// Read-only access to a remote work tracker, implemented once per provider.
#[async_trait]
pub trait WorkTracker: Send + Sync {
    async fn verify_connection(&self) -> Result<(), WorkTrackingError>;
    async fn list_projects(
        &self,
        page: PageRequest,
    ) -> Result<Page<WorkProject>, WorkTrackingError>;
    async fn list_groups(
        &self,
        project_id: &WorkProjectId,
        page: PageRequest,
    ) -> Result<Page<WorkItemGroup>, WorkTrackingError>;
    async fn list_items(
        &self,
        project_id: &WorkProjectId,
        group_id: &WorkItemGroupId,
        page: PageRequest,
    ) -> Result<Page<WorkItem>, WorkTrackingError>;
}

/// Synchronous because both backends (plugin store, keyring) are synchronous.
pub trait ConnectionRepository: Send + Sync {
    fn list(&self) -> Result<Vec<ProviderConnection>, WorkTrackingError>;
    fn find(&self, id: &ConnectionId) -> Result<Option<ProviderConnection>, WorkTrackingError>;
    /// Upsert by id.
    fn save(&self, connection: &ProviderConnection) -> Result<(), WorkTrackingError>;
    /// Idempotent.
    fn remove(&self, id: &ConnectionId) -> Result<(), WorkTrackingError>;
}

pub trait CredentialStore: Send + Sync {
    fn save(&self, id: &ConnectionId, key: &ApiKey) -> Result<(), WorkTrackingError>;
    fn read(&self, id: &ConnectionId) -> Result<Option<ApiKey>, WorkTrackingError>;
    /// Idempotent.
    fn delete(&self, id: &ConnectionId) -> Result<(), WorkTrackingError>;
}
