use serde::{Deserialize, Serialize};

use super::ai_tool::AiTool;

/// App-wide preferences, held as one record. Stored with field names so added
/// fields still read from older records, and every field has a default so a
/// record written before the field existed still loads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// Whether "Assign me to these tasks" is checked by default when creating
    /// contexts from tasks.
    pub assign_me_to_tasks: bool,
    /// The AI tool the app runs by default. Older records lack this field and
    /// load as [`AiTool::default`] (Kiro).
    pub default_ai_tool: AiTool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            assign_me_to_tasks: true,
            default_ai_tool: AiTool::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_record_without_the_default_ai_tool_loads_as_kiro() {
        let stored = json!({ "assignMeToTasks": false });

        let settings: AppSettings = serde_json::from_value(stored).unwrap();

        assert_eq!(settings.default_ai_tool, AiTool::Kiro);
    }
}
