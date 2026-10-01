//! Work Tracking bounded context: remote projects, item groups and items from
//! providers such as Teamwork (and later Jira or GitHub Issues).
//!
//! This module is the composition root: it is the only place outside
//! `infra::teamwork` that names the Teamwork adapter.
pub mod application;
pub mod commands;
pub mod domain;
pub mod infra;

use std::sync::Arc;

use application::registry::WorkTrackerRegistry;
use application::service::WorkTrackingService;
use domain::provider_kind::ProviderKind;
use infra::keychain_credential_store::KeychainCredentialStore;
use infra::plugin_store_connection_repository::PluginStoreConnectionRepository;

pub fn build_registry() -> WorkTrackerRegistry {
    WorkTrackerRegistry::new().register(ProviderKind::Teamwork, Arc::new(infra::teamwork::create))
}

pub fn build_service(app: tauri::AppHandle) -> WorkTrackingService {
    WorkTrackingService::new(
        Arc::new(PluginStoreConnectionRepository::new(app)),
        Arc::new(KeychainCredentialStore),
        build_registry(),
    )
}
