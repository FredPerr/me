//! Settings bounded context: app-wide preferences, starting with whether
//! "Assign me to these tasks" is checked by default.
//!
//! This module is the composition root: the only place that names the redb
//! settings store. It keeps one record in a `settings` table of the shared
//! `me.redb`, using the database handle passed in by the application setup.

pub mod application;
pub mod commands;
pub mod domain;
pub mod infra;

use std::sync::Arc;

use application::service::SettingsService;
use infra::redb_settings_store::RedbSettingsStore;

/// The Settings service, held in Tauri state.
pub struct Settings {
    pub service: Arc<SettingsService>,
}

pub fn build(database: Arc<redb::Database>) -> Result<Settings, Box<dyn std::error::Error>> {
    let store =
        RedbSettingsStore::new(database).map_err(|_| "the settings table could not be opened")?;
    let service = SettingsService::new(Arc::new(store));
    Ok(Settings {
        service: Arc::new(service),
    })
}
