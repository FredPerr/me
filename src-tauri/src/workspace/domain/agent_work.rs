use serde::{Deserialize, Serialize};

/// What an AI coding agent is doing in a context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWork {
    pub prompt: Option<String>,
    pub tool: AgentTool,
}

impl AgentWork {
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

/// The agent doing the work, with what it needs to resume its conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "camelCase")]
pub enum AgentTool {
    #[serde(rename_all = "camelCase")]
    Kiro { conversation_id: Option<String> },
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
