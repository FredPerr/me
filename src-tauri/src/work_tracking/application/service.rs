use std::sync::Arc;

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

use super::registry::WorkTrackerRegistry;
use crate::work_tracking::domain::api_key::ApiKey;
use crate::work_tracking::domain::assign_me::{AssignMeResult, FailedAssignment, TaskToAssign};
use crate::work_tracking::domain::base_url::BaseUrl;
use crate::work_tracking::domain::error::{InputField, WorkTrackingError};
use crate::work_tracking::domain::identifiers::{
    ConnectionId, RemoteProjectId, TaskGroupId, TaskId,
};
use crate::work_tracking::domain::page::{Cursor, Page, PageRequest, DEFAULT_PAGE_SIZE};
use crate::work_tracking::domain::pending_tasks::{
    CreatedFromTask, PendingTask, PendingTasks, ProjectTaskLink, ProjectTaskLinks,
};
use crate::work_tracking::domain::ports::{ConnectionRepository, CredentialStore, WorkTracker};
use crate::work_tracking::domain::provider_connection::{DisplayName, ProviderConnection};
use crate::work_tracking::domain::provider_kind::ProviderKind;
use crate::work_tracking::domain::remote_project::RemoteProject;
use crate::work_tracking::domain::task::{Task, TaskStatus};
use crate::work_tracking::domain::task_group::TaskGroup;

/// Open tasks of one remote project are paged until this many pages are read,
/// so a very long project cannot stall the board.
pub const MAX_PAGES_PER_PROJECT: usize = 20;

// No Debug derive: holds a raw secret until parsed.
pub struct SaveConnectionCommand {
    pub kind: String,
    pub base_url: String,
    pub display_name: Option<String>,
    pub api_key: String,
}

/// Connection as exposed to the frontend. Never contains the key.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub id: ConnectionId,
    pub kind: ProviderKind,
    pub base_url: String,
    pub display_name: String,
    pub credential_configured: bool,
}

impl ConnectionView {
    fn new(connection: &ProviderConnection, credential_configured: bool) -> Self {
        Self {
            id: connection.id().clone(),
            kind: connection.kind(),
            base_url: connection.base_url().as_str().to_string(),
            display_name: connection.display_name().as_str().to_string(),
            credential_configured,
        }
    }
}

pub struct WorkTrackingService {
    connections: Arc<dyn ConnectionRepository>,
    credentials: Arc<dyn CredentialStore>,
    registry: WorkTrackerRegistry,
    /// The remote project links of a local Project, read from `workspace`.
    project_links: Arc<dyn ProjectTaskLinks>,
    /// Caches decrypted API keys for the process lifetime so the OS keychain is
    /// read at most once per connection per run. Without this, every provider
    /// call reads the keychain, which on macOS can trigger a repeated unlock
    /// prompt. Invalidated when a connection is saved or removed.
    credential_cache: Mutex<HashMap<ConnectionId, ApiKey>>,
}

impl WorkTrackingService {
    pub fn new(
        connections: Arc<dyn ConnectionRepository>,
        credentials: Arc<dyn CredentialStore>,
        registry: WorkTrackerRegistry,
        project_links: Arc<dyn ProjectTaskLinks>,
    ) -> Self {
        Self {
            connections,
            credentials,
            registry,
            project_links,
            credential_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn list_connections(&self) -> Result<Vec<ConnectionView>, WorkTrackingError> {
        let connections = self.connections.list()?;
        Ok(connections
            .iter()
            .map(|connection| {
                // Prefer the in-memory cache so listing does not read the OS
                // keychain (which can prompt on macOS). A read error (e.g.
                // locked keychain) degrades this connection instead of failing
                // the call, so it can still be replaced or removed.
                let credential_configured = self
                    .credential_cache
                    .lock()
                    .unwrap()
                    .contains_key(connection.id())
                    || matches!(self.credentials.read(connection.id()), Ok(Some(_)));
                ConnectionView::new(connection, credential_configured)
            })
            .collect())
    }

    pub async fn save_connection(
        &self,
        command: SaveConnectionCommand,
    ) -> Result<ConnectionView, WorkTrackingError> {
        let kind = ProviderKind::parse(&command.kind)?;
        let base_url = BaseUrl::parse(&command.base_url)?;
        let display_name = DisplayName::parse(command.display_name.as_deref(), &base_url)?;
        let api_key = ApiKey::new(command.api_key)?;
        let connection = ProviderConnection::new(kind, base_url, display_name);

        // Verify before any write, so a rejected key persists nothing.
        self.registry
            .create(&connection, api_key.clone())?
            .verify_connection()
            .await?;

        let id = connection.id();
        let existed = self.connections.find(id)?.is_some();
        // Read the real stored key (not the cache) so the rollback below can
        // restore exactly what was in the keychain before this call.
        let previous_key = self.credentials.read(id)?;
        self.credentials.save(id, &api_key)?;
        if self.connections.save(&connection).is_err() {
            // Best-effort rollback so both stores end exactly as before the call.
            match (existed, &previous_key) {
                (true, Some(key)) => {
                    let _ = self.credentials.save(id, key);
                }
                _ => {
                    let _ = self.credentials.delete(id);
                }
            }
            self.invalidate_cached_credential(id);
            return Err(WorkTrackingError::StorageError);
        }
        // Cache the freshly saved key so the next call does not read the keychain.
        self.credential_cache
            .lock()
            .unwrap()
            .insert(id.clone(), api_key);
        Ok(ConnectionView::new(&connection, true))
    }

    pub fn remove_connection(&self, raw_connection_id: &str) -> Result<(), WorkTrackingError> {
        let id = ConnectionId::parse(raw_connection_id, InputField::ConnectionId)?;
        self.invalidate_cached_credential(&id);
        let removed = self.connections.remove(&id);
        let deleted = self.credentials.delete(&id);
        if removed.is_err() || deleted.is_err() {
            return Err(WorkTrackingError::StorageError);
        }
        Ok(())
    }

    pub async fn list_projects(
        &self,
        raw_connection_id: &str,
        raw_cursor: Option<&str>,
    ) -> Result<Page<RemoteProject>, WorkTrackingError> {
        let id = ConnectionId::parse(raw_connection_id, InputField::ConnectionId)?;
        let page = default_page(raw_cursor)?;
        self.resolve(&id)?.list_projects(page).await
    }

    pub async fn list_groups(
        &self,
        raw_connection_id: &str,
        raw_project_id: &str,
        raw_cursor: Option<&str>,
    ) -> Result<Page<TaskGroup>, WorkTrackingError> {
        let id = ConnectionId::parse(raw_connection_id, InputField::ConnectionId)?;
        let project_id = RemoteProjectId::parse(raw_project_id, InputField::ProjectId)?;
        let page = default_page(raw_cursor)?;
        self.resolve(&id)?.list_groups(&project_id, page).await
    }

    pub async fn list_items(
        &self,
        raw_connection_id: &str,
        raw_project_id: &str,
        raw_group_id: &str,
        raw_cursor: Option<&str>,
    ) -> Result<Page<Task>, WorkTrackingError> {
        let id = ConnectionId::parse(raw_connection_id, InputField::ConnectionId)?;
        let project_id = RemoteProjectId::parse(raw_project_id, InputField::ProjectId)?;
        let group_id = TaskGroupId::parse(raw_group_id, InputField::GroupId)?;
        let page = default_page(raw_cursor)?;
        self.resolve(&id)?
            .list_items(&project_id, &group_id, page)
            .await
    }

    pub async fn list_project_items(
        &self,
        raw_connection_id: &str,
        raw_project_id: &str,
        raw_cursor: Option<&str>,
    ) -> Result<Page<Task>, WorkTrackingError> {
        let id = ConnectionId::parse(raw_connection_id, InputField::ConnectionId)?;
        let project_id = RemoteProjectId::parse(raw_project_id, InputField::ProjectId)?;
        let page = default_page(raw_cursor)?;
        self.resolve(&id)?
            .list_project_items(&project_id, page)
            .await
    }

    /// Adds the authenticated user to a task's assignees, preserving the people
    /// already assigned. Resolves "me" first, then performs the additive update.
    pub async fn assign_me_to_task(
        &self,
        raw_connection_id: &str,
        raw_item_id: &str,
    ) -> Result<(), WorkTrackingError> {
        let id = ConnectionId::parse(raw_connection_id, InputField::ConnectionId)?;
        let item_id = TaskId::parse(raw_item_id, InputField::ItemId)?;
        self.assign_me_to_one(&id, &item_id).await
    }

    /// Best-effort assignment of the current user to each of the given tasks,
    /// used by the create-contexts-from-tasks flow after the contexts exist.
    /// A failure on one task never aborts the batch: the result lists the tasks
    /// that could not be assigned so the user can assign themselves by hand.
    /// An empty input assigns nothing and returns no failures.
    pub async fn assign_me_to_tasks(&self, tasks: &[TaskToAssign]) -> AssignMeResult {
        let mut result = AssignMeResult::default();
        for task in tasks {
            if self
                .assign_me_to_one(&task.connection_id, &task.task_id)
                .await
                .is_err()
            {
                result.failed.push(FailedAssignment {
                    connection_id: task.connection_id.clone(),
                    task_id: task.task_id.clone(),
                });
            }
        }
        result
    }

    /// Resolves "me" and performs the additive assignment for one already-parsed
    /// task. Shared by the single and batch assign-me paths.
    async fn assign_me_to_one(
        &self,
        connection_id: &ConnectionId,
        task_id: &TaskId,
    ) -> Result<(), WorkTrackingError> {
        let tracker = self.resolve(connection_id)?;
        let user_id = tracker.current_user_id().await?;
        tracker.assign_user_to_task(task_id, &user_id).await
    }

    /// The open tasks of a local Project's linked remote projects still waiting
    /// for a context: the board's Idle-column cards. For each link whose
    /// connection is configured, pages the remote project (all pages, capped at
    /// [`MAX_PAGES_PER_PROJECT`]), drops done and cancelled tasks, and drops
    /// tasks a context already came from. Links whose connection is missing,
    /// uncredentialed or unreachable are skipped and reported as unavailable so
    /// the user can reconnect. Tasks come back in link order.
    pub async fn pending_tasks(
        &self,
        project_tag: &str,
    ) -> Result<PendingTasks, WorkTrackingError> {
        let links = self.project_links.links_for(project_tag).await?;
        let mut pending = PendingTasks::default();
        for link in &links.links {
            match self.pending_tasks_of_link(link, &links.created_from).await {
                Ok(tasks) => pending.tasks.extend(tasks),
                // A missing, uncredentialed or unreachable connection is not an
                // error: its link stays, reported so the user can reconnect.
                Err(_) => {
                    if !pending
                        .unavailable_connections
                        .contains(&link.connection_id)
                    {
                        pending
                            .unavailable_connections
                            .push(link.connection_id.clone());
                    }
                }
            }
        }
        Ok(pending)
    }

    /// Pages one link's remote project and keeps only the pending tasks. Any
    /// resolve or fetch failure propagates so the caller marks the link
    /// unavailable.
    async fn pending_tasks_of_link(
        &self,
        link: &ProjectTaskLink,
        created_from: &std::collections::HashSet<CreatedFromTask>,
    ) -> Result<Vec<PendingTask>, WorkTrackingError> {
        let tracker = self.resolve(&link.connection_id)?;
        let mut pending = Vec::new();
        let mut cursor: Option<Cursor> = None;
        for _ in 0..MAX_PAGES_PER_PROJECT {
            let page = PageRequest::new(cursor.take(), DEFAULT_PAGE_SIZE)?;
            let fetched = tracker
                .list_project_items(&link.remote_project_id, page)
                .await?;
            for task in fetched.items {
                if is_pending(&task, &link.connection_id, created_from) {
                    pending.push(PendingTask {
                        connection_id: link.connection_id.clone(),
                        remote_project_id: link.remote_project_id.clone(),
                        task,
                    });
                }
            }
            match fetched.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        Ok(pending)
    }

    /// Builds a fresh adapter per call. The credential is served from the
    /// process-lifetime cache, falling back to the keychain on a miss, so the
    /// keychain is read at most once per connection per run.
    fn resolve(&self, id: &ConnectionId) -> Result<Box<dyn WorkTracker>, WorkTrackingError> {
        let connection = self
            .connections
            .find(id)?
            .ok_or(WorkTrackingError::NotConfigured)?;
        let key = self.cached_credential(id)?;
        self.registry.create(&connection, key)
    }

    /// Returns the API key from the in-memory cache, reading the keychain only
    /// on a miss and caching the result for subsequent calls.
    fn cached_credential(&self, id: &ConnectionId) -> Result<ApiKey, WorkTrackingError> {
        if let Some(key) = self.credential_cache.lock().unwrap().get(id).cloned() {
            return Ok(key);
        }
        let key = self
            .credentials
            .read(id)?
            .ok_or(WorkTrackingError::NotConfigured)?;
        self.credential_cache
            .lock()
            .unwrap()
            .insert(id.clone(), key.clone());
        Ok(key)
    }

    fn invalidate_cached_credential(&self, id: &ConnectionId) {
        self.credential_cache.lock().unwrap().remove(id);
    }
}

fn default_page(raw_cursor: Option<&str>) -> Result<PageRequest, WorkTrackingError> {
    let cursor = raw_cursor.map(Cursor::parse).transpose()?;
    PageRequest::new(cursor, DEFAULT_PAGE_SIZE)
}

/// A task belongs on the board when it is open (not done or cancelled) and no
/// context on this connection was already created from it.
fn is_pending(
    task: &Task,
    connection_id: &ConnectionId,
    created_from: &std::collections::HashSet<CreatedFromTask>,
) -> bool {
    if matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled) {
        return false;
    }
    let origin = CreatedFromTask {
        connection_id: connection_id.clone(),
        task_id: task.id.clone(),
    };
    !created_from.contains(&origin)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};

    use tauri::async_runtime::block_on;

    use super::*;
    use crate::work_tracking::application::test_support::{
        sample_connection, task_page, FakeConnectionRepository, FakeCredentialStore,
        FakeProjectTaskLinks, Script,
    };
    use crate::work_tracking::domain::error::InvalidInputReason;
    use crate::work_tracking::domain::identifiers::RemoteProjectId;
    use crate::work_tracking::domain::identifiers::TaskId;
    use crate::work_tracking::domain::pending_tasks::{
        CreatedFromTask, PendingTask, ProjectLinks, ProjectTaskLink,
    };
    use crate::work_tracking::domain::task::{Task, TaskPriority, TaskStatus};

    const CONNECTION_ID: &str = "teamwork:acme.teamwork.com";

    struct Fixture {
        connections: Arc<FakeConnectionRepository>,
        credentials: Arc<FakeCredentialStore>,
        script: Arc<Mutex<Script>>,
        project_links: Arc<FakeProjectTaskLinks>,
        service: WorkTrackingService,
    }

    fn fixture() -> Fixture {
        let connections = Arc::new(FakeConnectionRepository::default());
        let credentials = Arc::new(FakeCredentialStore::default());
        let script = Script::shared();
        let project_links = Arc::new(FakeProjectTaskLinks::default());
        let registry =
            WorkTrackerRegistry::new().register(ProviderKind::Teamwork, Script::factory(&script));
        let service = WorkTrackingService::new(
            connections.clone(),
            credentials.clone(),
            registry,
            project_links.clone(),
        );
        Fixture {
            connections,
            credentials,
            script,
            project_links,
            service,
        }
    }

    fn save_command(api_key: &str) -> SaveConnectionCommand {
        SaveConnectionCommand {
            kind: "teamwork".into(),
            base_url: "https://acme.teamwork.com/".into(),
            display_name: None,
            api_key: api_key.into(),
        }
    }

    #[test]
    fn save_success_persists_both() {
        let fixture = fixture();
        let view = block_on(fixture.service.save_connection(save_command("key-1"))).unwrap();
        assert_eq!(view.id.as_str(), CONNECTION_ID);
        assert_eq!(view.display_name, "acme.teamwork.com");
        assert_eq!(view.base_url, "https://acme.teamwork.com");
        assert!(view.credential_configured);
        assert_eq!(fixture.connections.stored(), vec![sample_connection()]);
        assert_eq!(
            fixture.credentials.stored(CONNECTION_ID),
            Some("key-1".into())
        );
        let script = fixture.script.lock().unwrap();
        assert_eq!(script.calls, vec!["verify_connection".to_string()]);
        assert_eq!(script.created_with_keys, vec!["key-1".to_string()]);
    }

    #[test]
    fn save_validates_base_url_before_api_key() {
        let fixture = fixture();
        let command = SaveConnectionCommand {
            kind: "teamwork".into(),
            base_url: "http://x".into(),
            display_name: None,
            api_key: String::new(),
        };
        let result = block_on(fixture.service.save_connection(command));
        assert_eq!(
            result,
            Err(WorkTrackingError::invalid_input(
                InputField::BaseUrl,
                InvalidInputReason::HttpsRequired
            ))
        );
    }

    #[test]
    fn save_validates_kind_first_and_api_key_last() {
        let fixture = fixture();
        let mut command = save_command("");
        command.kind = "jira".into();
        assert_eq!(
            block_on(fixture.service.save_connection(command)),
            Err(WorkTrackingError::invalid_input(
                InputField::ProviderKind,
                InvalidInputReason::Unsupported
            ))
        );
        let mut command = save_command("");
        command.display_name = Some("a".repeat(101));
        assert_eq!(
            block_on(fixture.service.save_connection(command)),
            Err(WorkTrackingError::invalid_input(
                InputField::DisplayName,
                InvalidInputReason::TooLong
            ))
        );
        assert_eq!(
            block_on(fixture.service.save_connection(save_command(""))),
            Err(WorkTrackingError::invalid_input(
                InputField::ApiKey,
                InvalidInputReason::Required
            ))
        );
    }

    #[test]
    fn verification_failure_persists_nothing() {
        let fixture = fixture();
        fixture.script.lock().unwrap().verify_result = Err(WorkTrackingError::Unauthorized);
        let result = block_on(fixture.service.save_connection(save_command("bad")));
        assert_eq!(result, Err(WorkTrackingError::Unauthorized));
        assert!(fixture.connections.stored().is_empty());
        assert_eq!(fixture.credentials.stored(CONNECTION_ID), None);
        assert_eq!(fixture.connections.save_calls.load(Ordering::SeqCst), 0);
        assert_eq!(fixture.credentials.save_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn credential_save_failure_skips_metadata_write() {
        let fixture = fixture();
        fixture.credentials.fail_save.store(true, Ordering::SeqCst);
        let result = block_on(fixture.service.save_connection(save_command("key-1")));
        assert_eq!(result, Err(WorkTrackingError::StorageError));
        assert_eq!(fixture.connections.save_calls.load(Ordering::SeqCst), 0);
        assert!(fixture.connections.stored().is_empty());
    }

    #[test]
    fn metadata_failure_on_new_connection_deletes_credential() {
        let fixture = fixture();
        fixture.connections.fail_save.store(true, Ordering::SeqCst);
        let result = block_on(fixture.service.save_connection(save_command("key-1")));
        assert_eq!(result, Err(WorkTrackingError::StorageError));
        assert_eq!(fixture.credentials.stored(CONNECTION_ID), None);
        assert!(fixture.connections.stored().is_empty());
    }

    #[test]
    fn metadata_failure_on_existing_connection_restores_previous_key() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "old-key");
        fixture.connections.fail_save.store(true, Ordering::SeqCst);
        let result = block_on(fixture.service.save_connection(save_command("new-key")));
        assert_eq!(result, Err(WorkTrackingError::StorageError));
        assert_eq!(
            fixture.credentials.stored(CONNECTION_ID),
            Some("old-key".into())
        );
        assert_eq!(fixture.connections.stored(), vec![sample_connection()]);
    }

    #[test]
    fn metadata_failure_on_existing_connection_without_key_deletes_credential() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.connections.fail_save.store(true, Ordering::SeqCst);
        let result = block_on(fixture.service.save_connection(save_command("new-key")));
        assert_eq!(result, Err(WorkTrackingError::StorageError));
        assert_eq!(fixture.credentials.stored(CONNECTION_ID), None);
        assert_eq!(fixture.connections.stored(), vec![sample_connection()]);
    }

    #[test]
    fn resave_replaces_key() {
        let fixture = fixture();
        block_on(fixture.service.save_connection(save_command("key-1"))).unwrap();
        block_on(fixture.service.save_connection(save_command("key-2"))).unwrap();
        assert_eq!(fixture.connections.stored().len(), 1);
        assert_eq!(
            fixture.credentials.stored(CONNECTION_ID),
            Some("key-2".into())
        );
    }

    #[test]
    fn remove_is_idempotent_and_attempts_both_stores() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        assert_eq!(fixture.service.remove_connection(CONNECTION_ID), Ok(()));
        assert!(fixture.connections.stored().is_empty());
        assert_eq!(fixture.credentials.stored(CONNECTION_ID), None);
        assert_eq!(fixture.service.remove_connection(CONNECTION_ID), Ok(()));

        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        fixture
            .connections
            .fail_remove
            .store(true, Ordering::SeqCst);
        assert_eq!(
            fixture.service.remove_connection(CONNECTION_ID),
            Err(WorkTrackingError::StorageError)
        );
        assert_eq!(fixture.credentials.stored(CONNECTION_ID), None);

        fixture
            .connections
            .fail_remove
            .store(false, Ordering::SeqCst);
        fixture
            .credentials
            .fail_delete
            .store(true, Ordering::SeqCst);
        assert_eq!(
            fixture.service.remove_connection(CONNECTION_ID),
            Err(WorkTrackingError::StorageError)
        );
        assert!(fixture.connections.stored().is_empty());
    }

    #[test]
    fn remove_rejects_invalid_id() {
        let fixture = fixture();
        assert_eq!(
            fixture.service.remove_connection(""),
            Err(WorkTrackingError::invalid_input(
                InputField::ConnectionId,
                InvalidInputReason::Required
            ))
        );
    }

    #[test]
    fn listing_requires_connection_and_credential() {
        let fixture = fixture();
        assert_eq!(
            block_on(fixture.service.list_projects(CONNECTION_ID, None)),
            Err(WorkTrackingError::NotConfigured)
        );
        fixture.connections.insert(sample_connection());
        assert_eq!(
            block_on(fixture.service.list_groups(CONNECTION_ID, "1", None)),
            Err(WorkTrackingError::NotConfigured)
        );
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        assert!(block_on(fixture.service.list_items(CONNECTION_ID, "1", "2", None)).is_ok());
    }

    #[test]
    fn listing_passes_scope_and_default_page() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        block_on(fixture.service.list_projects(CONNECTION_ID, Some("3"))).unwrap();
        block_on(fixture.service.list_groups(CONNECTION_ID, "10", None)).unwrap();
        block_on(
            fixture
                .service
                .list_items(CONNECTION_ID, "10", "20", Some("2")),
        )
        .unwrap();
        let script = fixture.script.lock().unwrap();
        assert_eq!(
            script.calls,
            vec![
                "list_projects cursor=Some(\"3\") size=50".to_string(),
                "list_groups project=10 cursor=None size=50".to_string(),
                "list_items project=10 group=20 cursor=Some(\"2\") size=50".to_string(),
            ]
        );
        assert_eq!(script.created_with_keys.len(), 3);
    }

    #[test]
    fn list_project_items_passes_project_scope() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        block_on(
            fixture
                .service
                .list_project_items(CONNECTION_ID, "10", Some("2")),
        )
        .unwrap();
        let script = fixture.script.lock().unwrap();
        assert_eq!(
            script.calls,
            vec!["list_project_items project=10 cursor=Some(\"2\") size=50".to_string()]
        );
    }

    #[test]
    fn list_project_items_requires_project_id() {
        let fixture = fixture();
        assert_eq!(
            block_on(fixture.service.list_project_items(CONNECTION_ID, "", None)),
            Err(WorkTrackingError::invalid_input(
                InputField::ProjectId,
                InvalidInputReason::Required
            ))
        );
    }

    #[test]
    fn credential_is_read_from_keychain_only_once_per_run() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        // Three provider calls that each resolve the adapter.
        block_on(fixture.service.list_projects(CONNECTION_ID, None)).unwrap();
        block_on(fixture.service.list_groups(CONNECTION_ID, "10", None)).unwrap();
        block_on(
            fixture
                .service
                .list_project_items(CONNECTION_ID, "10", None),
        )
        .unwrap();
        // The keychain was read once; later calls used the in-memory cache.
        assert_eq!(fixture.credentials.read_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn saving_a_connection_caches_the_new_key_without_rereading() {
        let fixture = fixture();
        block_on(fixture.service.save_connection(save_command("key-1"))).unwrap();
        let reads_after_save = fixture.credentials.read_calls.load(Ordering::SeqCst);
        block_on(fixture.service.list_projects(CONNECTION_ID, None)).unwrap();
        // Resolving after a save does not read the keychain again.
        assert_eq!(
            fixture.credentials.read_calls.load(Ordering::SeqCst),
            reads_after_save
        );
    }

    #[test]
    fn removing_a_connection_evicts_the_cached_key() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        // Prime the cache.
        block_on(fixture.service.list_projects(CONNECTION_ID, None)).unwrap();
        fixture.service.remove_connection(CONNECTION_ID).unwrap();
        // A later call with no connection must report NotConfigured, proving the
        // cached key was evicted rather than served stale.
        assert_eq!(
            block_on(fixture.service.list_projects(CONNECTION_ID, None)),
            Err(WorkTrackingError::NotConfigured)
        );
    }

    #[test]
    fn list_connections_uses_cache_when_available() {
        let fixture = fixture();
        block_on(fixture.service.save_connection(save_command("key-1"))).unwrap();
        let reads_before = fixture.credentials.read_calls.load(Ordering::SeqCst);
        let views = fixture.service.list_connections().unwrap();
        assert!(views[0].credential_configured);
        // The cached key answered credential_configured without a keychain read.
        assert_eq!(
            fixture.credentials.read_calls.load(Ordering::SeqCst),
            reads_before
        );
    }

    #[test]
    fn assign_me_resolves_user_then_assigns() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        fixture.script.lock().unwrap().current_user_result =
            Ok(crate::work_tracking::domain::identifiers::PersonId::from_numeric(55));
        assert_eq!(
            block_on(fixture.service.assign_me_to_task(CONNECTION_ID, "321")),
            Ok(())
        );
        let script = fixture.script.lock().unwrap();
        assert_eq!(
            script.calls,
            vec![
                "current_user_id".to_string(),
                "assign_user_to_task item=321 user=55".to_string(),
            ]
        );
    }

    #[test]
    fn assign_me_requires_connection_and_credential() {
        let fixture = fixture();
        assert_eq!(
            block_on(fixture.service.assign_me_to_task(CONNECTION_ID, "1")),
            Err(WorkTrackingError::NotConfigured)
        );
    }

    #[test]
    fn assign_me_validates_item_id() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        assert_eq!(
            block_on(fixture.service.assign_me_to_task(CONNECTION_ID, "")),
            Err(WorkTrackingError::invalid_input(
                InputField::ItemId,
                InvalidInputReason::Required
            ))
        );
    }

    #[test]
    fn assign_me_propagates_assign_failure() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        fixture.script.lock().unwrap().assign_result = Err(WorkTrackingError::Forbidden);
        assert_eq!(
            block_on(fixture.service.assign_me_to_task(CONNECTION_ID, "321")),
            Err(WorkTrackingError::Forbidden)
        );
    }

    fn task_to_assign(task_id: &str) -> TaskToAssign {
        TaskToAssign {
            connection_id: ConnectionId::parse(CONNECTION_ID, InputField::ConnectionId).unwrap(),
            task_id: TaskId::parse(task_id, InputField::ItemId).unwrap(),
        }
    }

    #[test]
    fn assign_me_to_tasks_assigns_every_task_on_success() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        let tasks = vec![task_to_assign("101"), task_to_assign("102")];

        let result = block_on(fixture.service.assign_me_to_tasks(&tasks));

        assert_eq!(result, AssignMeResult::default());
        let script = fixture.script.lock().unwrap();
        let assignments: Vec<_> = script
            .calls
            .iter()
            .filter(|call| call.starts_with("assign_user_to_task"))
            .collect();
        assert_eq!(assignments.len(), 2);
    }

    #[test]
    fn assign_me_to_tasks_reports_only_the_tasks_that_failed() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        fixture
            .script
            .lock()
            .unwrap()
            .failing_assign_tasks
            .insert("102".to_string());
        let tasks = vec![
            task_to_assign("101"),
            task_to_assign("102"),
            task_to_assign("103"),
        ];

        let result = block_on(fixture.service.assign_me_to_tasks(&tasks));

        // Only the scripted task is listed; the batch is not aborted.
        assert_eq!(
            result.failed,
            vec![FailedAssignment {
                connection_id: ConnectionId::parse(CONNECTION_ID, InputField::ConnectionId)
                    .unwrap(),
                task_id: TaskId::parse("102", InputField::ItemId).unwrap(),
            }]
        );
        let script = fixture.script.lock().unwrap();
        let assignments = script
            .calls
            .iter()
            .filter(|call| call.starts_with("assign_user_to_task"))
            .count();
        assert_eq!(assignments, 3);
    }

    #[test]
    fn assign_me_to_tasks_lists_a_task_whose_connection_is_missing() {
        let fixture = fixture();
        // No connection is configured, so resolving the adapter fails for the task.
        let tasks = vec![task_to_assign("101")];

        let result = block_on(fixture.service.assign_me_to_tasks(&tasks));

        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].task_id.as_str(), "101");
    }

    #[test]
    fn assign_me_to_tasks_assigns_nothing_for_an_empty_input() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");

        let result = block_on(fixture.service.assign_me_to_tasks(&[]));

        assert_eq!(result, AssignMeResult::default());
        assert!(fixture.script.lock().unwrap().calls.is_empty());
    }

    #[test]
    fn listing_validates_inputs() {
        let fixture = fixture();
        assert_eq!(
            block_on(fixture.service.list_groups(CONNECTION_ID, "", None)),
            Err(WorkTrackingError::invalid_input(
                InputField::ProjectId,
                InvalidInputReason::Required
            ))
        );
        assert_eq!(
            block_on(fixture.service.list_items(CONNECTION_ID, "1", "a/b", None)),
            Err(WorkTrackingError::invalid_input(
                InputField::GroupId,
                InvalidInputReason::InvalidFormat
            ))
        );
        assert_eq!(
            block_on(fixture.service.list_projects(CONNECTION_ID, Some(""))),
            Err(WorkTrackingError::invalid_input(
                InputField::Cursor,
                InvalidInputReason::InvalidFormat
            ))
        );
    }

    #[test]
    fn listing_propagates_adapter_errors() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        fixture.script.lock().unwrap().projects_result = Err(WorkTrackingError::RateLimited {
            retry_after_seconds: Some(5),
        });
        assert_eq!(
            block_on(fixture.service.list_projects(CONNECTION_ID, None)),
            Err(WorkTrackingError::RateLimited {
                retry_after_seconds: Some(5)
            })
        );
    }

    #[test]
    fn list_connections_reports_credential_state() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        let other = ProviderConnection::new(
            ProviderKind::Teamwork,
            BaseUrl::parse("https://other.teamwork.com").unwrap(),
            DisplayName::parse(
                Some("Other"),
                &BaseUrl::parse("https://other.teamwork.com").unwrap(),
            )
            .unwrap(),
        );
        fixture.connections.insert(other);
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        let views = fixture.service.list_connections().unwrap();
        assert_eq!(views.len(), 2);
        assert!(views[0].credential_configured);
        assert!(!views[1].credential_configured);
        assert_eq!(views[1].display_name, "Other");
    }

    #[test]
    fn list_connections_degrades_on_credential_read_error() {
        let fixture = fixture();
        fixture.connections.insert(sample_connection());
        fixture.credentials.insert(CONNECTION_ID, "key-1");
        fixture.credentials.fail_read.store(true, Ordering::SeqCst);
        let views = fixture.service.list_connections().unwrap();
        assert_eq!(views.len(), 1);
        assert!(!views[0].credential_configured);
    }

    #[test]
    fn list_connections_fails_when_repository_fails() {
        let fixture = fixture();
        fixture.connections.fail_list.store(true, Ordering::SeqCst);
        assert_eq!(
            fixture.service.list_connections(),
            Err(WorkTrackingError::StorageError)
        );
    }

    #[test]
    fn connection_view_never_contains_key() {
        let fixture = fixture();
        let secret = "twp_very_secret_key_123";
        let view = block_on(fixture.service.save_connection(save_command(secret))).unwrap();
        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.contains(secret), "{json}");
        assert_eq!(
            json,
            format!(
                r#"{{"id":"{CONNECTION_ID}","kind":"teamwork","baseUrl":"https://acme.teamwork.com","displayName":"acme.teamwork.com","credentialConfigured":true}}"#
            )
        );
        let listed = serde_json::to_string(&fixture.service.list_connections().unwrap()).unwrap();
        assert!(!listed.contains(secret));
    }

    mod pending_tasks {
        use super::*;

        const PROJECT_TAG: &str = "acme";
        const REMOTE_PROJECT_ID: &str = "42";
        const OTHER_REMOTE_PROJECT_ID: &str = "43";
        const OTHER_CONNECTION_ID: &str = "teamwork:other.teamwork.com";

        fn task(id: &str, status: TaskStatus) -> Task {
            Task {
                id: TaskId::from_numeric(id.parse().unwrap()),
                project_id: RemoteProjectId::parse(REMOTE_PROJECT_ID, InputField::ProjectId)
                    .unwrap(),
                group_id: None,
                parent_id: None,
                title: format!("Task {id}"),
                description: None,
                status,
                raw_status: "new".into(),
                priority: TaskPriority::None,
                raw_priority: None,
                assignees: vec![],
                labels: vec![],
                due_date: None,
                url: format!("https://acme.teamwork.com/app/tasks/{id}"),
                updated_at: None,
            }
        }

        fn link(connection_id: &str, remote_project_id: &str) -> ProjectTaskLink {
            ProjectTaskLink {
                connection_id: ConnectionId::parse(connection_id, InputField::ConnectionId)
                    .unwrap(),
                remote_project_id: RemoteProjectId::parse(remote_project_id, InputField::ProjectId)
                    .unwrap(),
            }
        }

        fn created_from(connection_id: &str, task_id: &str) -> CreatedFromTask {
            CreatedFromTask {
                connection_id: ConnectionId::parse(connection_id, InputField::ConnectionId)
                    .unwrap(),
                task_id: TaskId::from_numeric(task_id.parse().unwrap()),
            }
        }

        /// A configured connection with a stored credential.
        fn configured(fixture: &Fixture) {
            fixture.connections.insert(sample_connection());
            fixture.credentials.insert(CONNECTION_ID, "key-1");
        }

        fn ids(pending: &[PendingTask]) -> Vec<String> {
            pending
                .iter()
                .map(|task| task.task.id.as_str().to_string())
                .collect()
        }

        #[test]
        fn an_unknown_project_is_not_found() {
            let fixture = fixture();
            assert_eq!(
                block_on(fixture.service.pending_tasks(PROJECT_TAG)),
                Err(WorkTrackingError::NotFound)
            );
        }

        #[test]
        fn returns_nothing_for_a_project_without_links() {
            let fixture = fixture();
            fixture
                .project_links
                .set(PROJECT_TAG, Ok(ProjectLinks::default()));
            let pending = block_on(fixture.service.pending_tasks(PROJECT_TAG)).unwrap();
            assert_eq!(pending, PendingTasks::default());
        }

        #[test]
        fn pages_until_the_last_page_and_drops_done_and_cancelled() {
            let fixture = fixture();
            configured(&fixture);
            fixture.project_links.set(
                PROJECT_TAG,
                Ok(ProjectLinks {
                    links: vec![link(CONNECTION_ID, REMOTE_PROJECT_ID)],
                    created_from: HashSet::new(),
                }),
            );
            fixture.script.lock().unwrap().project_item_pages.insert(
                REMOTE_PROJECT_ID.into(),
                vec![
                    Ok(task_page(
                        vec![task("1", TaskStatus::Todo), task("2", TaskStatus::Done)],
                        Some(2),
                    )),
                    Ok(task_page(
                        vec![
                            task("3", TaskStatus::InProgress),
                            task("4", TaskStatus::Cancelled),
                        ],
                        None,
                    )),
                ],
            );

            let pending = block_on(fixture.service.pending_tasks(PROJECT_TAG)).unwrap();

            assert_eq!(ids(&pending.tasks), vec!["1", "3"]);
            assert!(pending.unavailable_connections.is_empty());
        }

        #[test]
        fn stops_after_the_page_cap() {
            let fixture = fixture();
            configured(&fixture);
            fixture.project_links.set(
                PROJECT_TAG,
                Ok(ProjectLinks {
                    links: vec![link(CONNECTION_ID, REMOTE_PROJECT_ID)],
                    created_from: HashSet::new(),
                }),
            );
            // Every page points to a next one, so only the cap stops paging.
            fixture.script.lock().unwrap().items_result =
                Ok(task_page(vec![task("1", TaskStatus::Todo)], Some(9)));

            block_on(fixture.service.pending_tasks(PROJECT_TAG)).unwrap();

            let item_calls = fixture
                .script
                .lock()
                .unwrap()
                .calls
                .iter()
                .filter(|call| call.starts_with("list_project_items"))
                .count();
            assert_eq!(item_calls, MAX_PAGES_PER_PROJECT);
        }

        #[test]
        fn drops_tasks_a_context_already_came_from() {
            let fixture = fixture();
            configured(&fixture);
            fixture.project_links.set(
                PROJECT_TAG,
                Ok(ProjectLinks {
                    links: vec![link(CONNECTION_ID, REMOTE_PROJECT_ID)],
                    created_from: HashSet::from([created_from(CONNECTION_ID, "1")]),
                }),
            );
            fixture.script.lock().unwrap().items_result = Ok(task_page(
                vec![task("1", TaskStatus::Todo), task("2", TaskStatus::Todo)],
                None,
            ));

            let pending = block_on(fixture.service.pending_tasks(PROJECT_TAG)).unwrap();

            assert_eq!(ids(&pending.tasks), vec!["2"]);
        }

        #[test]
        fn keeps_a_task_whose_created_from_is_on_another_connection() {
            let fixture = fixture();
            configured(&fixture);
            fixture.project_links.set(
                PROJECT_TAG,
                Ok(ProjectLinks {
                    links: vec![link(CONNECTION_ID, REMOTE_PROJECT_ID)],
                    // Same task id, but a different connection: still pending here.
                    created_from: HashSet::from([created_from(OTHER_CONNECTION_ID, "1")]),
                }),
            );
            fixture.script.lock().unwrap().items_result =
                Ok(task_page(vec![task("1", TaskStatus::Todo)], None));

            let pending = block_on(fixture.service.pending_tasks(PROJECT_TAG)).unwrap();

            assert_eq!(ids(&pending.tasks), vec!["1"]);
        }

        #[test]
        fn skips_and_reports_a_missing_or_uncredentialed_connection() {
            let fixture = fixture();
            // The connection metadata exists but has no stored credential.
            fixture.connections.insert(sample_connection());
            fixture.project_links.set(
                PROJECT_TAG,
                Ok(ProjectLinks {
                    links: vec![link(CONNECTION_ID, REMOTE_PROJECT_ID)],
                    created_from: HashSet::new(),
                }),
            );

            let pending = block_on(fixture.service.pending_tasks(PROJECT_TAG)).unwrap();

            assert!(pending.tasks.is_empty());
            assert_eq!(
                pending.unavailable_connections,
                vec![ConnectionId::parse(CONNECTION_ID, InputField::ConnectionId).unwrap()]
            );
        }

        #[test]
        fn reports_a_connection_whose_fetch_fails_as_unavailable() {
            let fixture = fixture();
            configured(&fixture);
            fixture.project_links.set(
                PROJECT_TAG,
                Ok(ProjectLinks {
                    links: vec![link(CONNECTION_ID, REMOTE_PROJECT_ID)],
                    created_from: HashSet::new(),
                }),
            );
            fixture.script.lock().unwrap().items_result = Err(WorkTrackingError::Timeout);

            let pending = block_on(fixture.service.pending_tasks(PROJECT_TAG)).unwrap();

            assert!(pending.tasks.is_empty());
            assert_eq!(
                pending.unavailable_connections,
                vec![ConnectionId::parse(CONNECTION_ID, InputField::ConnectionId).unwrap()]
            );
        }

        #[test]
        fn keeps_tasks_in_link_order_across_connections() {
            let fixture = fixture();
            // Both connections configured, each with its own remote project.
            fixture.connections.insert(sample_connection());
            fixture.credentials.insert(CONNECTION_ID, "key-1");
            let other = ProviderConnection::new(
                ProviderKind::Teamwork,
                BaseUrl::parse("https://other.teamwork.com").unwrap(),
                DisplayName::parse(None, &BaseUrl::parse("https://other.teamwork.com").unwrap())
                    .unwrap(),
            );
            fixture.connections.insert(other);
            fixture.credentials.insert(OTHER_CONNECTION_ID, "key-2");
            fixture.project_links.set(
                PROJECT_TAG,
                Ok(ProjectLinks {
                    links: vec![
                        link(CONNECTION_ID, REMOTE_PROJECT_ID),
                        link(OTHER_CONNECTION_ID, OTHER_REMOTE_PROJECT_ID),
                    ],
                    created_from: HashSet::new(),
                }),
            );
            {
                let mut script = fixture.script.lock().unwrap();
                script.project_item_pages.insert(
                    REMOTE_PROJECT_ID.into(),
                    vec![Ok(task_page(vec![task("1", TaskStatus::Todo)], None))],
                );
                script.project_item_pages.insert(
                    OTHER_REMOTE_PROJECT_ID.into(),
                    vec![Ok(task_page(vec![task("2", TaskStatus::Todo)], None))],
                );
            }

            let pending = block_on(fixture.service.pending_tasks(PROJECT_TAG)).unwrap();

            assert_eq!(ids(&pending.tasks), vec!["1", "2"]);
            let (first, second) = (&pending.tasks[0], &pending.tasks[1]);
            assert_eq!(first.remote_project_id.as_str(), REMOTE_PROJECT_ID);
            assert_eq!(second.connection_id.as_str(), OTHER_CONNECTION_ID);
        }
    }
}
