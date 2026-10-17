use serde::{Deserialize, Serialize};

/// App-wide preferences, held as one record. Stored with field names so added
/// fields still read from older records, and every field has a default so a
/// record written before the field existed still loads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// Whether "Assign me to these tasks" is checked by default when creating
    /// contexts from tasks.
    pub assign_me_to_tasks: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            assign_me_to_tasks: true,
        }
    }
}
