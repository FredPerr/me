use serde::{Deserialize, Serialize};

/// Where a context came from.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ContextOrigin {
    #[default]
    Manual,
    FromContext,
    FromBranches,
    FromPullRequests,
    FromTask(TaskRef),
}

/// The task, on a work tracker connection, a context was created from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRef {
    pub connection_id: String,
    pub task_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_a_task_origin_with_its_reference() {
        let origin = ContextOrigin::FromTask(TaskRef {
            connection_id: "teamwork:acme.teamwork.com".to_string(),
            task_id: "101".to_string(),
        });
        assert_eq!(
            serde_json::to_value(origin).unwrap(),
            serde_json::json!({
                "kind": "fromTask",
                "connectionId": "teamwork:acme.teamwork.com",
                "taskId": "101",
            })
        );
    }

    #[test]
    fn serializes_a_plain_origin_with_its_kind_only() {
        assert_eq!(
            serde_json::to_value(ContextOrigin::FromPullRequests).unwrap(),
            serde_json::json!({ "kind": "fromPullRequests" })
        );
    }
}
