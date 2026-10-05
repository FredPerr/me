use async_trait::async_trait;

use super::client::{
    paged_query, project_tasks_path, task_path, tasklists_path, tasks_path, TeamworkClient,
    ME_PATH, PROJECTS_PATH, PROJECTS_QUERY, TASKLISTS_QUERY, TASKS_QUERY,
};
use super::dto::{MeResponse, ProjectsResponse, TasklistsResponse, TasksResponse};
use super::mapper;
use crate::work_tracking::domain::base_url::BaseUrl;
use crate::work_tracking::domain::error::{InputField, WorkTrackingError};
use crate::work_tracking::domain::identifiers::{
    ConnectionId, PersonId, WorkItemGroupId, WorkItemId, WorkProjectId,
};
use crate::work_tracking::domain::page::{Page, PageRequest};
use crate::work_tracking::domain::ports::WorkTracker;
use crate::work_tracking::domain::work_item::WorkItem;
use crate::work_tracking::domain::work_item_group::WorkItemGroup;
use crate::work_tracking::domain::work_project::WorkProject;

pub struct TeamworkWorkTracker {
    client: TeamworkClient,
    connection_id: ConnectionId,
    base_url: BaseUrl,
}

impl TeamworkWorkTracker {
    pub fn new(client: TeamworkClient, connection_id: ConnectionId, base_url: BaseUrl) -> Self {
        Self {
            client,
            connection_id,
            base_url,
        }
    }
}

#[async_trait]
impl WorkTracker for TeamworkWorkTracker {
    async fn verify_connection(&self) -> Result<(), WorkTrackingError> {
        let _: MeResponse = self.client.get_json(ME_PATH, &[]).await?;
        Ok(())
    }

    async fn current_user_id(&self) -> Result<PersonId, WorkTrackingError> {
        let response: MeResponse = self.client.get_json(ME_PATH, &[]).await?;
        Ok(mapper::me_person_id(&response))
    }

    async fn assign_user_to_task(
        &self,
        item_id: &WorkItemId,
        user_id: &PersonId,
    ) -> Result<(), WorkTrackingError> {
        let item_segment = mapper::numeric_id(item_id.as_str(), InputField::ItemId)?;
        let body = mapper::add_assignee_body(user_id);
        // The response envelope is irrelevant on success; discard it.
        let _: serde::de::IgnoredAny = self.client.patch_json(&task_path(item_segment), &body).await?;
        Ok(())
    }

    async fn list_projects(
        &self,
        page: PageRequest,
    ) -> Result<Page<WorkProject>, WorkTrackingError> {
        let page_number = mapper::page_number(page.cursor())?;
        let query = paged_query(PROJECTS_QUERY, page_number, page.page_size());
        let response: ProjectsResponse = self.client.get_json(PROJECTS_PATH, &query).await?;
        Ok(mapper::map_projects(
            response,
            &self.connection_id,
            &self.base_url,
            page_number,
        ))
    }

    async fn list_groups(
        &self,
        project_id: &WorkProjectId,
        page: PageRequest,
    ) -> Result<Page<WorkItemGroup>, WorkTrackingError> {
        let project_segment = mapper::numeric_id(project_id.as_str(), InputField::ProjectId)?;
        let page_number = mapper::page_number(page.cursor())?;
        let query = paged_query(TASKLISTS_QUERY, page_number, page.page_size());
        let response: TasklistsResponse = self
            .client
            .get_json(&tasklists_path(project_segment), &query)
            .await?;
        Ok(mapper::map_tasklists(
            response,
            project_id,
            &self.base_url,
            page_number,
        ))
    }

    async fn list_items(
        &self,
        project_id: &WorkProjectId,
        group_id: &WorkItemGroupId,
        page: PageRequest,
    ) -> Result<Page<WorkItem>, WorkTrackingError> {
        mapper::numeric_id(project_id.as_str(), InputField::ProjectId)?;
        let group_segment = mapper::numeric_id(group_id.as_str(), InputField::GroupId)?;
        let page_number = mapper::page_number(page.cursor())?;
        let query = paged_query(TASKS_QUERY, page_number, page.page_size());
        let response: TasksResponse = self
            .client
            .get_json(&tasks_path(group_segment), &query)
            .await?;
        Ok(mapper::map_tasks(
            response,
            project_id,
            Some(group_id),
            &self.base_url,
            page_number,
        ))
    }

    async fn list_project_items(
        &self,
        project_id: &WorkProjectId,
        page: PageRequest,
    ) -> Result<Page<WorkItem>, WorkTrackingError> {
        let project_segment = mapper::numeric_id(project_id.as_str(), InputField::ProjectId)?;
        let page_number = mapper::page_number(page.cursor())?;
        let query = paged_query(TASKS_QUERY, page_number, page.page_size());
        let response: TasksResponse = self
            .client
            .get_json(&project_tasks_path(project_segment), &query)
            .await?;
        Ok(mapper::map_tasks(
            response,
            project_id,
            None,
            &self.base_url,
            page_number,
        ))
    }
}

#[cfg(test)]
mod tests {
    use tauri::async_runtime::block_on;

    use super::*;
    use crate::work_tracking::domain::api_key::ApiKey;
    use crate::work_tracking::domain::error::InvalidInputReason;
    use crate::work_tracking::domain::page::Cursor;

    // Every case below fails validation before any request is sent.
    fn tracker() -> TeamworkWorkTracker {
        let base_url = BaseUrl::parse("https://acme.teamwork.com").unwrap();
        let client = TeamworkClient::new(&base_url, ApiKey::new("key".into()).unwrap()).unwrap();
        let connection_id =
            ConnectionId::parse("teamwork:acme.teamwork.com", InputField::ConnectionId).unwrap();
        TeamworkWorkTracker::new(client, connection_id, base_url)
    }

    fn first_page() -> PageRequest {
        PageRequest::new(None, 50).unwrap()
    }

    #[test]
    fn rejects_non_digit_ids_before_requesting() {
        let tracker = tracker();
        let project = WorkProjectId::parse("abc", InputField::ProjectId).unwrap();
        let group = WorkItemGroupId::parse("12", InputField::GroupId).unwrap();
        assert_eq!(
            block_on(tracker.list_groups(&project, first_page())),
            Err(WorkTrackingError::invalid_input(
                InputField::ProjectId,
                InvalidInputReason::InvalidFormat
            ))
        );
        assert_eq!(
            block_on(tracker.list_items(&project, &group, first_page())),
            Err(WorkTrackingError::invalid_input(
                InputField::ProjectId,
                InvalidInputReason::InvalidFormat
            ))
        );
        assert_eq!(
            block_on(tracker.list_project_items(&project, first_page())),
            Err(WorkTrackingError::invalid_input(
                InputField::ProjectId,
                InvalidInputReason::InvalidFormat
            ))
        );
        let valid_project = WorkProjectId::parse("1", InputField::ProjectId).unwrap();
        let bad_group = WorkItemGroupId::parse("x1", InputField::GroupId).unwrap();
        assert_eq!(
            block_on(tracker.list_items(&valid_project, &bad_group, first_page())),
            Err(WorkTrackingError::invalid_input(
                InputField::GroupId,
                InvalidInputReason::InvalidFormat
            ))
        );
    }

    #[test]
    fn rejects_non_numeric_cursor_before_requesting() {
        let tracker = tracker();
        let page = PageRequest::new(Some(Cursor::parse("next").unwrap()), 50).unwrap();
        assert_eq!(
            block_on(tracker.list_projects(page)),
            Err(WorkTrackingError::invalid_input(
                InputField::Cursor,
                InvalidInputReason::InvalidFormat
            ))
        );
    }
}
