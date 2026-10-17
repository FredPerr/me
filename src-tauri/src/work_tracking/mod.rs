//! Work Tracking bounded context: remote projects, task groups and tasks from
//! providers such as Teamwork (and later Jira or GitHub Issues).
//!
//! This module is the composition root: it is the only place outside
//! `infra::teamwork` that names the Teamwork adapter, and the only place that
//! names the keychain and the redb connection store.
pub mod application;
pub mod commands;
pub mod domain;
pub mod infra;

use std::path::Path;
use std::sync::Arc;

use application::registry::WorkTrackerRegistry;
use application::service::WorkTrackingService;
use domain::provider_kind::ProviderKind;
use infra::connection_import::import_legacy_connections;
use infra::keychain_credential_store::KeychainCredentialStore;
use infra::redb_connection_repository::RedbConnectionRepository;

pub fn build_registry() -> WorkTrackerRegistry {
    WorkTrackerRegistry::new().register(ProviderKind::Teamwork, Arc::new(infra::teamwork::create))
}

/// Builds the service on the shared redb handle, importing any connections left
/// in the old `work-tracking.json` in `app_data_dir` on first run. API keys stay
/// in the keychain, untouched by the import.
pub fn build_service(
    database: Arc<redb::Database>,
    app_data_dir: &Path,
) -> Result<WorkTrackingService, Box<dyn std::error::Error>> {
    let connections = Arc::new(
        RedbConnectionRepository::new(database)
            .map_err(|_| "the work tracking connection table could not be opened")?,
    );
    if let Err(error) = import_legacy_connections(app_data_dir, connections.as_ref()) {
        // A failed import should not stop the app from starting; the user can
        // reconnect by hand. The file is only renamed after a successful read.
        eprintln!("[work_tracking] legacy connection import failed: {error}");
    }
    Ok(WorkTrackingService::new(
        connections,
        Arc::new(KeychainCredentialStore),
        build_registry(),
    ))
}
