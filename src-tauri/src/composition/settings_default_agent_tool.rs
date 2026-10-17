//! `workspace`'s `DefaultAgentTool`, read from the `settings` service. The only
//! place `workspace`'s default-tool port meets `settings`: `workspace` depends
//! on the port, `settings` owns the preference, and this adapter maps the
//! stored [`AiTool`] to the parallel [`AgentTool`] a fresh context's first run
//! starts on (#90).
//!
//! The only tool today is Kiro. When the settings cannot be read the default
//! (Kiro) is used — the tool is always available, so a read failure never
//! blocks a run.

use std::sync::Arc;

use crate::settings::application::service::SettingsService;
use crate::settings::domain::ai_tool::AiTool;
use crate::workspace::domain::agent_work::AgentTool;
use crate::workspace::domain::ports::DefaultAgentTool;

pub struct SettingsDefaultAgentTool {
    settings: Arc<SettingsService>,
}

impl SettingsDefaultAgentTool {
    pub fn new(settings: Arc<SettingsService>) -> Self {
        Self { settings }
    }
}

impl DefaultAgentTool for SettingsDefaultAgentTool {
    fn default_tool(&self) -> AgentTool {
        let tool = self
            .settings
            .get()
            .map(|settings| settings.default_ai_tool)
            .unwrap_or_default();
        agent_tool_for(tool)
    }
}

/// Maps the settings `AiTool` to the parallel domain `AgentTool`, as a fresh
/// tool with no conversation recorded yet.
fn agent_tool_for(tool: AiTool) -> AgentTool {
    match tool {
        AiTool::Kiro => AgentTool::Kiro {
            conversation_id: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::infra::in_memory_settings_store::InMemorySettingsStore;

    #[test]
    fn maps_the_default_kiro_setting_to_the_kiro_agent_tool() {
        let service = SettingsService::new(Arc::new(InMemorySettingsStore::default()));
        let default_tool = SettingsDefaultAgentTool::new(Arc::new(service));

        assert_eq!(
            default_tool.default_tool(),
            AgentTool::Kiro {
                conversation_id: None
            }
        );
    }
}
