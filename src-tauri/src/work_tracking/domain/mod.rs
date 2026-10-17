//! Provider-agnostic model of the Work Tracking context. Depends only on
//! `std`, `serde` and `async_trait`.
pub mod api_key;
pub mod assign_me;
pub mod base_url;
pub mod error;
pub mod identifiers;
pub mod label;
pub mod page;
pub mod pending_tasks;
pub mod person;
pub mod ports;
pub mod provider_connection;
pub mod provider_kind;
pub mod remote_project;
pub mod task;
pub mod task_group;

#[cfg(test)]
mod serialization_tests {
    use super::identifiers::PersonId;
    use super::identifiers::{RemoteProjectId, TaskId};
    use super::label::Label;
    use super::page::{Cursor, Page};
    use super::person::Person;
    use super::task::{Task, TaskPriority, TaskStatus};

    fn minimal_task() -> Task {
        Task {
            id: TaskId::from_numeric(1),
            project_id: RemoteProjectId::from_numeric(2),
            group_id: None,
            parent_id: None,
            title: "Task".into(),
            description: None,
            status: TaskStatus::InProgress,
            raw_status: "new".into(),
            priority: TaskPriority::None,
            raw_priority: None,
            assignees: vec![Person::new(PersonId::from_numeric(3), "Ada".into(), None)],
            labels: vec![Label::new("bug".into(), None)],
            due_date: None,
            url: "https://acme.teamwork.com/app/tasks/1".into(),
            updated_at: None,
        }
    }

    #[test]
    fn task_emits_absent_optionals_as_null() {
        // Wire types keep a stable shape for specta: every optional is present,
        // null when absent, rather than omitted.
        let json = serde_json::to_string(&minimal_task()).unwrap();
        for key in [
            "groupId",
            "parentId",
            "description",
            "rawPriority",
            "dueDate",
            "updatedAt",
        ] {
            assert!(
                json.contains(&format!("\"{key}\":null")),
                "{key} not null in {json}"
            );
        }
        assert!(json.contains("\"avatarUrl\":null"));
        assert!(json.contains("\"color\":null"));
        assert!(json.contains("\"status\":\"inProgress\""));
        assert!(json.contains("\"priority\":\"none\""));
        assert!(json.contains("\"projectId\":\"2\""));
    }

    #[test]
    fn typed_id_serializes_as_bare_string() {
        assert_eq!(
            serde_json::to_string(&TaskId::from_numeric(42)).unwrap(),
            "\"42\""
        );
    }

    #[test]
    fn page_cursor_is_bare_string_or_null() {
        let without_cursor: Page<u8> = Page {
            items: vec![],
            next_cursor: None,
        };
        assert_eq!(
            serde_json::to_string(&without_cursor).unwrap(),
            r#"{"items":[],"nextCursor":null}"#
        );
        let with_cursor: Page<u8> = Page {
            items: vec![1],
            next_cursor: Some(Cursor::from_numeric(2)),
        };
        let json = serde_json::to_string(&with_cursor).unwrap();
        assert!(json.contains(r#""nextCursor":"2""#), "{json}");
    }

    #[test]
    fn person_keeps_only_https_avatar() {
        let insecure = Person::new(
            PersonId::from_numeric(1),
            "A".into(),
            Some("http://x".into()),
        );
        assert!(serde_json::to_string(&insecure)
            .unwrap()
            .contains("\"avatarUrl\":null"));
        let secure = Person::new(
            PersonId::from_numeric(1),
            "A".into(),
            Some("https://x".into()),
        );
        assert!(serde_json::to_string(&secure)
            .unwrap()
            .contains(r#""avatarUrl":"https://x""#));
    }
}
