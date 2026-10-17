use serde::{Deserialize, Serialize};

/// Which AI tool the app runs by default. One variant for now (Kiro); the type
/// is an enum so adding a tool later is a variant, not a string contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiTool {
    #[default]
    Kiro,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_default_tool_is_kiro() {
        assert_eq!(AiTool::default(), AiTool::Kiro);
    }

    #[test]
    fn kiro_serializes_in_camel_case() {
        assert_eq!(serde_json::to_value(AiTool::Kiro).unwrap(), json!("kiro"));
    }

    #[test]
    fn kiro_deserializes_from_its_wire_name() {
        let tool: AiTool = serde_json::from_value(json!("kiro")).unwrap();

        assert_eq!(tool, AiTool::Kiro);
    }
}
