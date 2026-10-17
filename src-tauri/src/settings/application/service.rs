use std::sync::Arc;

use crate::settings::domain::app_settings::AppSettings;
use crate::settings::domain::ports::{SettingsStore, SettingsStoreUnavailable};

/// Reads and updates the app-wide settings through the store.
pub struct SettingsService {
    store: Arc<dyn SettingsStore>,
}

impl SettingsService {
    pub fn new(store: Arc<dyn SettingsStore>) -> Self {
        Self { store }
    }

    /// The current settings, or the defaults when none are stored.
    pub fn get(&self) -> Result<AppSettings, SettingsStoreUnavailable> {
        self.store.load()
    }

    /// Sets whether "Assign me to these tasks" is checked by default, leaving
    /// the other settings as they are.
    pub fn set_assign_me_to_tasks(
        &self,
        value: bool,
    ) -> Result<AppSettings, SettingsStoreUnavailable> {
        let mut settings = self.store.load()?;
        settings.assign_me_to_tasks = value;
        self.store.save(&settings)?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::infra::in_memory_settings_store::InMemorySettingsStore;

    fn service_with_empty_store() -> SettingsService {
        SettingsService::new(Arc::new(InMemorySettingsStore::default()))
    }

    #[test]
    fn get_returns_the_defaults_on_a_fresh_store() {
        let service = service_with_empty_store();

        assert_eq!(service.get().unwrap(), AppSettings::default());
    }

    #[test]
    fn set_assign_me_to_tasks_then_get_round_trips() {
        let service = service_with_empty_store();

        service.set_assign_me_to_tasks(false).unwrap();

        assert!(!service.get().unwrap().assign_me_to_tasks);
    }

    #[test]
    fn set_assign_me_to_tasks_returns_the_updated_settings() {
        let service = service_with_empty_store();

        let updated = service.set_assign_me_to_tasks(false).unwrap();

        assert_eq!(
            updated,
            AppSettings {
                assign_me_to_tasks: false,
            }
        );
    }
}
