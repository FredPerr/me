//! Anti-corruption layer: pure Teamwork DTO → domain mapping.
use super::dto::{
    IncludedDto, MetaDto, ProjectDto, ProjectsResponse, TaskDto, TasklistDto, TasklistsResponse,
    TasksResponse,
};
use super::web_urls;
use crate::work_tracking::domain::base_url::BaseUrl;
use crate::work_tracking::domain::error::{InputField, InvalidInputReason, WorkTrackingError};
use crate::work_tracking::domain::identifiers::{
    ConnectionId, PersonId, WorkItemGroupId, WorkItemId, WorkProjectId,
};
use crate::work_tracking::domain::label::Label;
use crate::work_tracking::domain::page::{Cursor, Page};
use crate::work_tracking::domain::person::Person;
use crate::work_tracking::domain::work_item::{WorkItem, WorkItemPriority, WorkItemStatus};
use crate::work_tracking::domain::work_item_group::{WorkItemGroup, WorkItemGroupKind};
use crate::work_tracking::domain::work_project::{WorkProject, WorkProjectStatus};

const MAX_NUMERIC_ID_DIGITS: usize = 20;
const DATE_LENGTH: usize = 10;

/// Teamwork IDs are integers; anything else is rejected before building a path.
pub fn numeric_id(raw: &str, field: InputField) -> Result<&str, WorkTrackingError> {
    let is_numeric = (1..=MAX_NUMERIC_ID_DIGITS).contains(&raw.len())
        && raw.bytes().all(|byte| byte.is_ascii_digit());
    if is_numeric {
        Ok(raw)
    } else {
        Err(WorkTrackingError::invalid_input(
            field,
            InvalidInputReason::InvalidFormat,
        ))
    }
}

/// Cursor in → Teamwork page number (`None` → first page).
pub fn page_number(cursor: Option<&Cursor>) -> Result<u32, WorkTrackingError> {
    let Some(cursor) = cursor else {
        return Ok(1);
    };
    let raw = cursor.as_str();
    let parsed = raw
        .bytes()
        .all(|byte| byte.is_ascii_digit())
        .then(|| raw.parse::<u32>().ok())
        .flatten();
    match parsed {
        Some(page) if page >= 1 => Ok(page),
        _ => Err(WorkTrackingError::invalid_input(
            InputField::Cursor,
            InvalidInputReason::InvalidFormat,
        )),
    }
}

fn next_cursor(page: u32, meta: Option<&MetaDto>) -> Option<Cursor> {
    let has_more = meta
        .and_then(|meta| meta.page.as_ref())
        .and_then(|page_meta| page_meta.has_more)
        .unwrap_or(false);
    if !has_more {
        return None;
    }
    page.checked_add(1)
        .map(|next| Cursor::from_numeric(u64::from(next)))
}

fn project_status(raw: Option<&str>) -> WorkProjectStatus {
    match raw {
        Some("active") => WorkProjectStatus::Active,
        // In v3 "inactive" is the archived state.
        Some("inactive") => WorkProjectStatus::Archived,
        _ => WorkProjectStatus::Other,
    }
}

/// v3 exposes "started" only through `progress`, so a new or reopened task
/// with progress is reported as in progress.
fn task_status(raw: Option<&str>, progress: Option<u32>) -> WorkItemStatus {
    match raw {
        Some("new") | Some("reopened") if progress.unwrap_or(0) > 0 => WorkItemStatus::InProgress,
        Some("new") | Some("reopened") => WorkItemStatus::Todo,
        Some("completed") => WorkItemStatus::Done,
        Some("deleted") => WorkItemStatus::Cancelled,
        _ => WorkItemStatus::Unknown,
    }
}

fn task_priority(raw: Option<&str>) -> WorkItemPriority {
    match raw.map(str::to_ascii_lowercase).as_deref() {
        None | Some("") => WorkItemPriority::None,
        Some("low") => WorkItemPriority::Low,
        Some("medium") => WorkItemPriority::Medium,
        Some("high") => WorkItemPriority::High,
        Some(_) => WorkItemPriority::Unknown,
    }
}

fn due_date(raw: Option<&str>) -> Option<String> {
    let date = raw?.get(..DATE_LENGTH)?;
    let matches_pattern = date.bytes().enumerate().all(|(index, byte)| match index {
        4 | 7 => byte == b'-',
        _ => byte.is_ascii_digit(),
    });
    matches_pattern.then(|| date.to_string())
}

fn fallback_name(id: u64) -> String {
    format!("#{id}")
}

fn assignees(ids: &[u64], included: &IncludedDto) -> Vec<Person> {
    ids.iter()
        .map(|id| {
            let user = included.users.get(&id.to_string());
            let full_name = user
                .map(|user| {
                    format!(
                        "{} {}",
                        user.first_name.as_deref().unwrap_or_default(),
                        user.last_name.as_deref().unwrap_or_default()
                    )
                    .trim()
                    .to_string()
                })
                .unwrap_or_default();
            let display_name = if full_name.is_empty() {
                fallback_name(*id)
            } else {
                full_name
            };
            let avatar_url = user.and_then(|user| user.avatar_url.clone());
            Person::new(PersonId::from_numeric(*id), display_name, avatar_url)
        })
        .collect()
}

fn labels(ids: &[u64], included: &IncludedDto) -> Vec<Label> {
    ids.iter()
        .map(|id| match included.tags.get(&id.to_string()) {
            Some(tag) => Label::new(tag.name.clone(), tag.color.clone()),
            None => Label::new(fallback_name(*id), None),
        })
        .collect()
}

fn map_project(
    project: ProjectDto,
    connection_id: &ConnectionId,
    base_url: &BaseUrl,
) -> WorkProject {
    WorkProject {
        id: WorkProjectId::from_numeric(project.id),
        connection_id: connection_id.clone(),
        name: project.name,
        description: project.description,
        status: project_status(project.status.as_deref()),
        url: web_urls::project_url(base_url, project.id),
    }
}

fn map_tasklist(
    tasklist: TasklistDto,
    project_id: &WorkProjectId,
    base_url: &BaseUrl,
) -> WorkItemGroup {
    WorkItemGroup {
        id: WorkItemGroupId::from_numeric(tasklist.id),
        project_id: project_id.clone(),
        name: tasklist.name,
        description: tasklist.description,
        kind: WorkItemGroupKind::TaskList,
        position: tasklist.display_order,
        url: Some(web_urls::tasklist_url(base_url, tasklist.id)),
    }
}

fn map_task(
    task: TaskDto,
    included: &IncludedDto,
    project_id: &WorkProjectId,
    fallback_group_id: Option<&WorkItemGroupId>,
    base_url: &BaseUrl,
) -> WorkItem {
    WorkItem {
        id: WorkItemId::from_numeric(task.id),
        project_id: project_id.clone(),
        group_id: task
            .tasklist_id
            .map(WorkItemGroupId::from_numeric)
            .or_else(|| fallback_group_id.cloned()),
        parent_id: task
            .parent_task_id
            .filter(|parent| *parent != 0)
            .map(WorkItemId::from_numeric),
        title: task.name,
        description: task.description,
        status: task_status(task.status.as_deref(), task.progress),
        raw_status: task.status.clone().unwrap_or_default(),
        priority: task_priority(task.priority.as_deref()),
        raw_priority: task.priority.clone(),
        assignees: assignees(&task.assignee_user_ids, included),
        labels: labels(&task.tag_ids, included),
        due_date: due_date(task.due_date.as_deref()),
        url: web_urls::task_url(base_url, task.id),
        updated_at: task.updated_at,
    }
}

pub fn map_projects(
    response: ProjectsResponse,
    connection_id: &ConnectionId,
    base_url: &BaseUrl,
    page: u32,
) -> Page<WorkProject> {
    Page {
        next_cursor: next_cursor(page, response.meta.as_ref()),
        items: response
            .projects
            .into_iter()
            .map(|project| map_project(project, connection_id, base_url))
            .collect(),
    }
}

pub fn map_tasklists(
    response: TasklistsResponse,
    project_id: &WorkProjectId,
    base_url: &BaseUrl,
    page: u32,
) -> Page<WorkItemGroup> {
    Page {
        next_cursor: next_cursor(page, response.meta.as_ref()),
        items: response
            .tasklists
            .into_iter()
            .map(|tasklist| map_tasklist(tasklist, project_id, base_url))
            .collect(),
    }
}

pub fn map_tasks(
    response: TasksResponse,
    project_id: &WorkProjectId,
    fallback_group_id: Option<&WorkItemGroupId>,
    base_url: &BaseUrl,
    page: u32,
) -> Page<WorkItem> {
    let included = response.included.unwrap_or_default();
    Page {
        next_cursor: next_cursor(page, response.meta.as_ref()),
        items: response
            .tasks
            .into_iter()
            .map(|task| map_task(task, &included, project_id, fallback_group_id, base_url))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;

    use super::*;

    const PROJECTS_PAGE: &str = include_str!("fixtures/projects_page.json");
    const TASKLISTS_PAGE: &str = include_str!("fixtures/tasklists_page.json");
    const TASKS_PAGE: &str = include_str!("fixtures/tasks_page.json");
    const TASKS_MISSING_SIDELOADS: &str = include_str!("fixtures/tasks_missing_sideloads.json");
    const TASKS_UNKNOWN_VALUES: &str = include_str!("fixtures/tasks_unknown_values.json");
    const TASKS_NULL_COLLECTIONS: &str = include_str!("fixtures/tasks_null_collections.json");

    fn parse<T: DeserializeOwned>(json: &str) -> T {
        serde_json::from_str(json).expect("fixture parses")
    }

    fn base_url() -> BaseUrl {
        BaseUrl::parse("https://acme.teamwork.com").unwrap()
    }

    fn connection_id() -> ConnectionId {
        ConnectionId::parse("teamwork:acme.teamwork.com", InputField::ConnectionId).unwrap()
    }

    fn project_id() -> WorkProjectId {
        WorkProjectId::from_numeric(100)
    }

    fn group_id() -> WorkItemGroupId {
        WorkItemGroupId::from_numeric(200)
    }

    fn tasks(json: &str, page: u32) -> Page<WorkItem> {
        map_tasks(
            parse(json),
            &project_id(),
            Some(&group_id()),
            &base_url(),
            page,
        )
    }

    fn task(page: &Page<WorkItem>, id: u64) -> &WorkItem {
        let id = WorkItemId::from_numeric(id);
        page.items
            .iter()
            .find(|item| item.id == id)
            .expect("task present")
    }

    #[test]
    fn maps_projects_and_statuses() {
        let page = map_projects(parse(PROJECTS_PAGE), &connection_id(), &base_url(), 1);
        let statuses: Vec<WorkProjectStatus> =
            page.items.iter().map(|project| project.status).collect();
        assert_eq!(
            statuses,
            vec![
                WorkProjectStatus::Active,
                WorkProjectStatus::Archived,
                WorkProjectStatus::Other,
                WorkProjectStatus::Other,
            ]
        );
        let first = &page.items[0];
        assert_eq!(first.id.as_str(), "101");
        assert_eq!(first.name, "Website");
        assert_eq!(first.description.as_deref(), Some("Marketing site"));
        assert_eq!(first.connection_id, connection_id());
        assert_eq!(first.url, "https://acme.teamwork.com/app/projects/101");
        assert_eq!(page.items[3].description, None);
        assert_eq!(page.next_cursor, Some(Cursor::from_numeric(2)));
    }

    #[test]
    fn maps_tasklists() {
        let page = map_tasklists(parse(TASKLISTS_PAGE), &project_id(), &base_url(), 3);
        assert_eq!(page.items.len(), 2);
        let first = &page.items[0];
        assert_eq!(first.id.as_str(), "201");
        assert_eq!(first.project_id, project_id());
        assert_eq!(first.kind, WorkItemGroupKind::TaskList);
        assert_eq!(first.position, Some(2));
        assert_eq!(
            first.url.as_deref(),
            Some("https://acme.teamwork.com/app/tasklists/201")
        );
        assert_eq!(page.items[1].position, None);
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn maps_task_statuses() {
        let page = tasks(TASKS_PAGE, 1);
        assert_eq!(task(&page, 1).status, WorkItemStatus::Todo);
        assert_eq!(task(&page, 2).status, WorkItemStatus::InProgress);
        assert_eq!(task(&page, 3).status, WorkItemStatus::Done);
        assert_eq!(task(&page, 4).status, WorkItemStatus::Cancelled);
        assert_eq!(task(&page, 5).status, WorkItemStatus::InProgress);
        assert_eq!(task(&page, 5).raw_status, "reopened");
        assert_eq!(task(&page, 6).status, WorkItemStatus::Todo);
    }

    #[test]
    fn maps_task_priorities() {
        let page = tasks(TASKS_PAGE, 1);
        assert_eq!(task(&page, 1).priority, WorkItemPriority::High);
        assert_eq!(task(&page, 1).raw_priority.as_deref(), Some("high"));
        assert_eq!(task(&page, 2).priority, WorkItemPriority::Medium);
        assert_eq!(task(&page, 3).priority, WorkItemPriority::Low);
        assert_eq!(task(&page, 4).priority, WorkItemPriority::None);
        assert_eq!(task(&page, 4).raw_priority, None);
        assert_eq!(task(&page, 5).priority, WorkItemPriority::None);
        assert_eq!(task(&page, 6).priority, WorkItemPriority::High);
    }

    #[test]
    fn maps_unknown_values() {
        let page = tasks(TASKS_UNKNOWN_VALUES, 1);
        let unknown = task(&page, 10);
        assert_eq!(unknown.status, WorkItemStatus::Unknown);
        assert_eq!(unknown.raw_status, "blocked");
        assert_eq!(unknown.priority, WorkItemPriority::Unknown);
        assert_eq!(unknown.raw_priority.as_deref(), Some("critical"));
        let missing = task(&page, 11);
        assert_eq!(missing.status, WorkItemStatus::Unknown);
        assert_eq!(missing.raw_status, "");
        assert_eq!(missing.due_date, None);
    }

    #[test]
    fn maps_assignees_in_order_with_fallback() {
        let page = tasks(TASKS_PAGE, 1);
        assert_eq!(
            task(&page, 1).assignees,
            vec![
                Person::new(
                    PersonId::from_numeric(301),
                    "Ada Lovelace".into(),
                    Some("https://cdn.teamwork.com/ada.png".into())
                ),
                Person::new(PersonId::from_numeric(302), "Grace".into(), None),
                Person::new(PersonId::from_numeric(399), "#399".into(), None),
            ]
        );
        assert_eq!(
            task(&page, 2).assignees,
            vec![Person::new(
                PersonId::from_numeric(303),
                "#303".into(),
                None
            )]
        );
    }

    #[test]
    fn maps_labels_with_color_validation() {
        let page = tasks(TASKS_PAGE, 1);
        assert_eq!(
            task(&page, 1).labels,
            vec![
                Label::new("bug".into(), Some("#d84640".into())),
                Label::new("ui".into(), None),
                Label::new("#499".into(), None),
            ]
        );
    }

    #[test]
    fn missing_sideloads_fall_back_to_ids() {
        let page = tasks(TASKS_MISSING_SIDELOADS, 1);
        let item = task(&page, 20);
        assert_eq!(
            item.assignees,
            vec![Person::new(
                PersonId::from_numeric(301),
                "#301".into(),
                None
            )]
        );
        assert_eq!(item.labels, vec![Label::new("#401".into(), None)]);
    }

    #[test]
    fn maps_parent_group_dates_and_urls() {
        let page = tasks(TASKS_PAGE, 1);
        let parent = task(&page, 1);
        assert_eq!(parent.parent_id, None);
        assert_eq!(parent.group_id, Some(WorkItemGroupId::from_numeric(201)));
        assert_eq!(parent.project_id, project_id());
        assert_eq!(parent.due_date.as_deref(), Some("2024-01-15"));
        assert_eq!(parent.updated_at.as_deref(), Some("2024-01-10T12:00:00Z"));
        assert_eq!(parent.url, "https://acme.teamwork.com/app/tasks/1");
        assert_eq!(parent.title, "Design homepage");
        assert_eq!(parent.description.as_deref(), Some("Hero section"));

        let subtask = task(&page, 2);
        assert_eq!(subtask.parent_id, Some(WorkItemId::from_numeric(1)));
        assert_eq!(subtask.due_date.as_deref(), Some("2024-02-01"));

        assert_eq!(task(&page, 3).parent_id, None);
        assert_eq!(task(&page, 3).group_id, Some(group_id()));
        assert_eq!(task(&page, 4).due_date, None);
        assert_eq!(task(&page, 5).due_date, None);
    }

    #[test]
    fn has_more_produces_next_cursor() {
        assert_eq!(
            tasks(TASKS_PAGE, 4).next_cursor,
            Some(Cursor::from_numeric(5))
        );
        assert_eq!(tasks(TASKS_MISSING_SIDELOADS, 1).next_cursor, None);
        assert_eq!(tasks(TASKS_UNKNOWN_VALUES, 1).next_cursor, None);
        assert_eq!(tasks(TASKS_PAGE, u32::MAX).next_cursor, None);
    }

    #[test]
    fn null_collections_map_to_empty() {
        let variants: Vec<serde_json::Value> = parse(TASKS_NULL_COLLECTIONS);
        assert_eq!(variants.len(), 2);
        for variant in variants {
            let page = tasks(&variant.to_string(), 1);
            assert_eq!(page.items.len(), 1);
            assert!(page.items[0].assignees.is_empty());
            assert!(page.items[0].labels.is_empty());
            assert_eq!(page.next_cursor, None);
        }
    }

    #[test]
    fn null_envelopes_map_to_empty_pages() {
        let projects = map_projects(
            parse(r#"{"projects":null}"#),
            &connection_id(),
            &base_url(),
            1,
        );
        assert!(projects.items.is_empty());
        let tasklists = map_tasklists(
            parse(r#"{"tasklists":null,"meta":null}"#),
            &project_id(),
            &base_url(),
            1,
        );
        assert!(tasklists.items.is_empty());
        let page = tasks(r#"{"tasks":null,"included":null}"#, 1);
        assert!(page.items.is_empty());
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn malformed_body_fails_to_parse() {
        assert!(serde_json::from_str::<TasksResponse>(r#"{"tasks":[{"id":"x"}]}"#).is_err());
        assert!(serde_json::from_str::<ProjectsResponse>("<html></html>").is_err());
        assert!(serde_json::from_str::<super::super::dto::MeResponse>(r#"{"other":1}"#).is_err());
        assert!(
            serde_json::from_str::<super::super::dto::MeResponse>(r#"{"person":{"id":1}}"#).is_ok()
        );
    }

    #[test]
    fn numeric_id_rules() {
        assert_eq!(numeric_id("123", InputField::ProjectId), Ok("123"));
        assert!(numeric_id(&"9".repeat(20), InputField::GroupId).is_ok());
        for raw in ["", "abc", "12a", "-1", "+1", &"9".repeat(21)] {
            assert_eq!(
                numeric_id(raw, InputField::ProjectId),
                Err(WorkTrackingError::invalid_input(
                    InputField::ProjectId,
                    InvalidInputReason::InvalidFormat
                )),
                "{raw}"
            );
        }
    }

    #[test]
    fn page_number_rules() {
        assert_eq!(page_number(None), Ok(1));
        assert_eq!(page_number(Some(&Cursor::from_numeric(7))), Ok(7));
        let invalid = Err(WorkTrackingError::invalid_input(
            InputField::Cursor,
            InvalidInputReason::InvalidFormat,
        ));
        for raw in ["0", "abc", "+2", "4294967296"] {
            assert_eq!(
                page_number(Some(&Cursor::parse(raw).unwrap())),
                invalid,
                "{raw}"
            );
        }
    }
}
