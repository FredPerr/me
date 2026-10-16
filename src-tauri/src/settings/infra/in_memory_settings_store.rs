use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::settings::domain::app_settings::AppSettings;
use crate::settings::domain::ports::{SettingsStore, SettingsStoreUnavailable};

/// Settings kept in memory, defaulting when none were saved, like redb.
#[derive(Default)]
pub struct InMemorySettingsStore {
    settings: Mutex<Option<AppSettings>>,
}

impl InMemorySettingsStore {
    fn settings(&self) -> MutexGuard<'_, Option<AppSettings>> {
        // Every write replaces the whole record, so a poisoned cell is still consistent.
        self.settings.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl SettingsStore for InMemorySettingsStore {
    fn load(&self) -> Result<AppSettings, SettingsStoreUnavailable> {
        Ok(self.settings().clone().unwrap_or_default())
    }

    fn save(&self, settings: &AppSettings) -> Result<(), SettingsStoreUnavailable> {
        *self.settings() = Some(settings.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::infra::settings_store_contract::settings_store_contract_tests;

    fn with_store(test: fn(&dyn SettingsStore)) {
        test(&InMemorySettingsStore::default());
    }

    settings_store_contract_tests!(with_store);
}
