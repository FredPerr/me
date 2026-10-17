//! `ai_assist`'s `DefaultAiTool`, read from the `settings` service. The only
//! place `ai_assist`'s default-tool port meets `settings`: `ai_assist` depends
//! on the port, `settings` owns the preference, and this adapter maps the
//! stored [`AiTool`] to the adapter id the runner spawns.
//!
//! The only tool today is Kiro, whose `ai_session` adapter id is `"kiro"`. When
//! the settings cannot be read the default (Kiro) is used — the tool is always
//! available, so a read failure never blocks a suggestion.

use std::sync::Arc;

use crate::ai_assist::domain::ports::DefaultAiTool;
use crate::settings::application::service::SettingsService;
use crate::settings::domain::ai_tool::AiTool;

/// The `ai_session` adapter id of the Kiro tool.
const KIRO_ADAPTER_ID: &str = "kiro";

pub struct SettingsDefaultAiTool {
    settings: Arc<SettingsService>,
}

impl SettingsDefaultAiTool {
    pub fn new(settings: Arc<SettingsService>) -> Self {
        Self { settings }
    }
}

impl DefaultAiTool for SettingsDefaultAiTool {
    fn tool_id(&self) -> String {
        let tool = self
            .settings
            .get()
            .map(|settings| settings.default_ai_tool)
            .unwrap_or_default();
        adapter_id(tool).to_string()
    }
}

/// Maps a tool to its `ai_session` adapter id.
fn adapter_id(tool: AiTool) -> &'static str {
    match tool {
        AiTool::Kiro => KIRO_ADAPTER_ID,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::infra::in_memory_settings_store::InMemorySettingsStore;

    #[test]
    fn maps_the_default_kiro_tool_to_its_adapter_id() {
        let service = SettingsService::new(Arc::new(InMemorySettingsStore::default()));
        let default_tool = SettingsDefaultAiTool::new(Arc::new(service));

        assert_eq!(default_tool.tool_id(), KIRO_ADAPTER_ID);
    }
}
