use serde::{Deserialize, Serialize};

/// Where a context stands in the flow of work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextStatus {
    /// The default context's status, which it never leaves.
    Idle,
    /// The status a new context starts in.
    #[default]
    Ready,
    InProgress,
    NeedsHuman,
    Review,
    Done,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_context_starts_ready() {
        assert_eq!(ContextStatus::default(), ContextStatus::Ready);
    }

    #[test]
    fn serializes_with_the_frontend_names() {
        let statuses = [
            (ContextStatus::Idle, "\"idle\""),
            (ContextStatus::Ready, "\"ready\""),
            (ContextStatus::InProgress, "\"inProgress\""),
            (ContextStatus::NeedsHuman, "\"needsHuman\""),
            (ContextStatus::Review, "\"review\""),
            (ContextStatus::Done, "\"done\""),
        ];
        for (status, expected) in statuses {
            assert_eq!(serde_json::to_string(&status).unwrap(), expected);
        }
    }
}
