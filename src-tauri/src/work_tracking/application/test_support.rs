//! In-memory fakes for the application-layer tests. Each test owns its own
//! instances (no statics), so tests run in parallel.
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::registry::WorkTrackerFactory;
use crate::work_tracking::domain::api_key::ApiKey;
use crate::work_tracking::domain::base_url::BaseUrl;
use crate::work_tracking::domain::error::WorkTrackingError;
use crate::work_tracking::domain::identifiers::{
    ConnectionId, PersonId, WorkItemGroupId, WorkItemId, WorkProjectId,
};
use crate::work_tracking::domain::page::{Page, PageRequest};
use crate::work_tracking::domain::ports::{ConnectionRepository, CredentialStore, WorkTracker};
use crate::work_tracking::domain::provider_connection::{DisplayName, ProviderConnection};
use crate::work_tracking::domain::provider_kind::ProviderKind;
use crate::work_tracking::domain::work_item::WorkItem;
use crate::work_tracking::domain::work_item_group::WorkItemGroup;
use crate::work_tracking::domain::work_project::WorkProject;

pub fn sample_connection() -> ProviderConnection {
    let base_url = BaseUrl::parse("https://acme.teamwork.com").unwrap();
    let display_name = DisplayName::parse(None, &base_url).unwrap();
    ProviderConnection::new(ProviderKind::Teamwork, base_url, display_name)
}

fn empty_page<T>() -> Page<T> {
    Page {
        items: Vec::new(),
        next_cursor: None,
    }
}

/// Scripted adapter results plus a log of factory and adapter calls.
pub struct Script {
    pub verify_result: Result<(), WorkTrackingError>,
    pub projects_result: Result<Page<WorkProject>, WorkTrackingError>,
    pub groups_result: Result<Page<WorkItemGroup>, WorkTrackingError>,
    pub items_result: Result<Page<WorkItem>, WorkTrackingError>,
    pub current_user_result: Result<PersonId, WorkTrackingError>,
    pub assign_result: Result<(), WorkTrackingError>,
    pub created_with_keys: Vec<String>,
    pub calls: Vec<String>,
}

impl Script {
    pub fn shared() -> Arc<Mutex<Script>> {
        Arc::new(Mutex::new(Script {
            verify_result: Ok(()),
            projects_result: Ok(empty_page()),
            groups_result: Ok(empty_page()),
            items_result: Ok(empty_page()),
            current_user_result: Ok(PersonId::from_numeric(7)),
            assign_result: Ok(()),
            created_with_keys: Vec::new(),
            calls: Vec::new(),
        }))
    }

    pub fn factory(script: &Arc<Mutex<Script>>) -> WorkTrackerFactory {
        let script = script.clone();
        Arc::new(move |_connection: &ProviderConnection, key: ApiKey| {
            script
                .lock()
                .unwrap()
                .created_with_keys
                .push(key.expose().to_string());
            Ok(Box::new(FakeWorkTracker {
                script: script.clone(),
            }) as Box<dyn WorkTracker>)
        })
    }
}

struct FakeWorkTracker {
    script: Arc<Mutex<Script>>,
}

fn describe(page: &PageRequest) -> String {
    format!(
        "cursor={:?} size={}",
        page.cursor().map(|cursor| cursor.as_str()),
        page.page_size()
    )
}

#[async_trait]
impl WorkTracker for FakeWorkTracker {
    async fn verify_connection(&self) -> Result<(), WorkTrackingError> {
        let mut script = self.script.lock().unwrap();
        script.calls.push("verify_connection".into());
        script.verify_result.clone()
    }

    async fn current_user_id(&self) -> Result<PersonId, WorkTrackingError> {
        let mut script = self.script.lock().unwrap();
        script.calls.push("current_user_id".into());
        script.current_user_result.clone()
    }

    async fn assign_user_to_task(
        &self,
        item_id: &WorkItemId,
        user_id: &PersonId,
    ) -> Result<(), WorkTrackingError> {
        let mut script = self.script.lock().unwrap();
        script.calls.push(format!(
            "assign_user_to_task item={} user={}",
            item_id.as_str(),
            user_id.as_str()
        ));
        script.assign_result.clone()
    }

    async fn list_projects(
        &self,
        page: PageRequest,
    ) -> Result<Page<WorkProject>, WorkTrackingError> {
        let mut script = self.script.lock().unwrap();
        script
            .calls
            .push(format!("list_projects {}", describe(&page)));
        script.projects_result.clone()
    }

    async fn list_groups(
        &self,
        project_id: &WorkProjectId,
        page: PageRequest,
    ) -> Result<Page<WorkItemGroup>, WorkTrackingError> {
        let mut script = self.script.lock().unwrap();
        script.calls.push(format!(
            "list_groups project={} {}",
            project_id.as_str(),
            describe(&page)
        ));
        script.groups_result.clone()
    }

    async fn list_items(
        &self,
        project_id: &WorkProjectId,
        group_id: &WorkItemGroupId,
        page: PageRequest,
    ) -> Result<Page<WorkItem>, WorkTrackingError> {
        let mut script = self.script.lock().unwrap();
        script.calls.push(format!(
            "list_items project={} group={} {}",
            project_id.as_str(),
            group_id.as_str(),
            describe(&page)
        ));
        script.items_result.clone()
    }

    async fn list_project_items(
        &self,
        project_id: &WorkProjectId,
        page: PageRequest,
    ) -> Result<Page<WorkItem>, WorkTrackingError> {
        let mut script = self.script.lock().unwrap();
        script.calls.push(format!(
            "list_project_items project={} {}",
            project_id.as_str(),
            describe(&page)
        ));
        script.items_result.clone()
    }
}

#[derive(Default)]
pub struct FakeConnectionRepository {
    connections: Mutex<Vec<ProviderConnection>>,
    pub fail_list: AtomicBool,
    pub fail_save: AtomicBool,
    pub fail_remove: AtomicBool,
    pub save_calls: AtomicUsize,
}

impl FakeConnectionRepository {
    pub fn insert(&self, connection: ProviderConnection) {
        self.connections.lock().unwrap().push(connection);
    }

    pub fn stored(&self) -> Vec<ProviderConnection> {
        self.connections.lock().unwrap().clone()
    }
}

impl ConnectionRepository for FakeConnectionRepository {
    fn list(&self) -> Result<Vec<ProviderConnection>, WorkTrackingError> {
        if self.fail_list.load(Ordering::SeqCst) {
            return Err(WorkTrackingError::StorageError);
        }
        Ok(self.stored())
    }

    fn find(&self, id: &ConnectionId) -> Result<Option<ProviderConnection>, WorkTrackingError> {
        Ok(self
            .stored()
            .into_iter()
            .find(|connection| connection.id() == id))
    }

    fn save(&self, connection: &ProviderConnection) -> Result<(), WorkTrackingError> {
        self.save_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_save.load(Ordering::SeqCst) {
            return Err(WorkTrackingError::StorageError);
        }
        let mut connections = self.connections.lock().unwrap();
        connections.retain(|existing| existing.id() != connection.id());
        connections.push(connection.clone());
        Ok(())
    }

    fn remove(&self, id: &ConnectionId) -> Result<(), WorkTrackingError> {
        if self.fail_remove.load(Ordering::SeqCst) {
            return Err(WorkTrackingError::StorageError);
        }
        self.connections
            .lock()
            .unwrap()
            .retain(|connection| connection.id() != id);
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeCredentialStore {
    keys: Mutex<HashMap<String, String>>,
    pub fail_save: AtomicBool,
    pub fail_read: AtomicBool,
    pub fail_delete: AtomicBool,
    pub save_calls: AtomicUsize,
    pub read_calls: AtomicUsize,
}

impl FakeCredentialStore {
    pub fn insert(&self, id: &str, key: &str) {
        self.keys.lock().unwrap().insert(id.into(), key.into());
    }

    pub fn stored(&self, id: &str) -> Option<String> {
        self.keys.lock().unwrap().get(id).cloned()
    }
}

impl CredentialStore for FakeCredentialStore {
    fn save(&self, id: &ConnectionId, key: &ApiKey) -> Result<(), WorkTrackingError> {
        self.save_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_save.load(Ordering::SeqCst) {
            return Err(WorkTrackingError::StorageError);
        }
        self.insert(id.as_str(), key.expose());
        Ok(())
    }

    fn read(&self, id: &ConnectionId) -> Result<Option<ApiKey>, WorkTrackingError> {
        self.read_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_read.load(Ordering::SeqCst) {
            return Err(WorkTrackingError::StorageError);
        }
        Ok(self
            .stored(id.as_str())
            .map(|key| ApiKey::new(key).unwrap()))
    }

    fn delete(&self, id: &ConnectionId) -> Result<(), WorkTrackingError> {
        if self.fail_delete.load(Ordering::SeqCst) {
            return Err(WorkTrackingError::StorageError);
        }
        self.keys.lock().unwrap().remove(id.as_str());
        Ok(())
    }
}
