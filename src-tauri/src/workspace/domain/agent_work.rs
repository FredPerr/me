use serde::{Deserialize, Serialize};

/// What an AI coding agent is doing in a context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWork {
    pub prompt: Option<String>,
    pub tool: AgentTool,
}

impl AgentWork {
    /// A fresh work on `tool`, with no prompt and no conversation yet, for a
    /// context's first run.
    pub fn on_tool(tool: AgentTool) -> Self {
        Self {
            prompt: None,
            tool: tool.fresh(),
        }
    }

    pub fn with_prompt(self, prompt: Option<String>) -> Self {
        Self {
            prompt: prompt.filter(|text| !text.trim().is_empty()),
            ..self
        }
    }

    pub fn with_conversation(self, conversation_id: String) -> Self {
        match self.tool {
            AgentTool::Kiro { .. } => Self {
                tool: AgentTool::Kiro {
                    conversation_id: Some(conversation_id),
                },
                ..self
            },
        }
    }
}

impl Default for AgentWork {
    fn default() -> Self {
        Self {
            prompt: None,
            tool: AgentTool::Kiro {
                conversation_id: None,
            },
        }
    }
}

/// The `ai_session` adapter id of the Kiro tool, the id the runner spawns.
const KIRO_ADAPTER_ID: &str = "kiro";

/// The agent doing the work, with what it needs to resume its conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "camelCase")]
pub enum AgentTool {
    #[serde(rename_all = "camelCase")]
    Kiro { conversation_id: Option<String> },
}

impl AgentTool {
    /// A fresh tool with no conversation yet, ready to record on a context's
    /// first run so follow-up runs resume it.
    pub fn fresh(&self) -> Self {
        match self {
            Self::Kiro { .. } => Self::Kiro {
                conversation_id: None,
            },
        }
    }

    /// The `ai_session` adapter id the runner spawns for this tool. The single
    /// source mapping a context's recorded tool to its adapter id, so adding a
    /// tool later is one new arm here.
    pub fn adapter_id(&self) -> &'static str {
        match self {
            Self::Kiro { .. } => KIRO_ADAPTER_ID,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROMPT: &str = "Fix the login page";
    const CONVERSATION_ID: &str = "conv-123";

    #[test]
    fn keeps_a_prompt() {
        let work = AgentWork::default().with_prompt(Some(PROMPT.to_string()));
        assert_eq!(work.prompt.as_deref(), Some(PROMPT));
    }

    #[test]
    fn treats_a_blank_prompt_as_no_prompt() {
        let work = AgentWork::default().with_prompt(Some("  ".to_string()));
        assert_eq!(work.prompt, None);
    }

    #[test]
    fn records_the_conversation_and_keeps_the_prompt() {
        let work = AgentWork::default()
            .with_prompt(Some(PROMPT.to_string()))
            .with_conversation(CONVERSATION_ID.to_string());
        assert_eq!(
            work.tool,
            AgentTool::Kiro {
                conversation_id: Some(CONVERSATION_ID.to_string())
            }
        );
        assert_eq!(work.prompt.as_deref(), Some(PROMPT));
    }

    #[test]
    fn kiro_maps_to_its_adapter_id() {
        let tool = AgentTool::Kiro {
            conversation_id: None,
        };
        assert_eq!(tool.adapter_id(), "kiro");
    }

    #[test]
    fn a_fresh_tool_keeps_the_kind_and_drops_the_conversation() {
        let tool = AgentTool::Kiro {
            conversation_id: Some(CONVERSATION_ID.to_string()),
        };
        assert_eq!(
            tool.fresh(),
            AgentTool::Kiro {
                conversation_id: None
            }
        );
    }

    #[test]
    fn serializes_the_tool_as_tagged_json() {
        let work = AgentWork::default().with_conversation(CONVERSATION_ID.to_string());
        assert_eq!(
            serde_json::to_value(work).unwrap(),
            serde_json::json!({
                "prompt": null,
                "tool": { "tool": "kiro", "conversationId": CONVERSATION_ID },
            })
        );
    }
}
