use std::sync::Arc;

use crate::settings::domain::ai_tool::AiTool;
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

    /// Sets the AI tool the app runs by default, leaving the other settings as
    /// they are.
    pub fn set_default_ai_tool(
        &self,
        tool: AiTool,
    ) -> Result<AppSettings, SettingsStoreUnavailable> {
        let mut settings = self.store.load()?;
        settings.default_ai_tool = tool;
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
                default_ai_tool: AiTool::Kiro,
            }
        );
    }

    #[test]
    fn set_default_ai_tool_then_get_round_trips() {
        let service = service_with_empty_store();

        service.set_default_ai_tool(AiTool::Kiro).unwrap();

        assert_eq!(service.get().unwrap().default_ai_tool, AiTool::Kiro);
    }

    #[test]
    fn set_default_ai_tool_returns_the_updated_settings() {
        let service = service_with_empty_store();

        let updated = service.set_default_ai_tool(AiTool::Kiro).unwrap();

        assert_eq!(updated.default_ai_tool, AiTool::Kiro);
    }

    #[test]
    fn set_default_ai_tool_leaves_assign_me_to_tasks_untouched() {
        let service = service_with_empty_store();
        service.set_assign_me_to_tasks(false).unwrap();

        let updated = service.set_default_ai_tool(AiTool::Kiro).unwrap();

        assert!(!updated.assign_me_to_tasks);
    }
}
