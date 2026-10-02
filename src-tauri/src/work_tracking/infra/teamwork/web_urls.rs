//! Teamwork web app URLs. Not part of the API docs; kept here so any
//! correction found during manual verification stays in one place.
use crate::work_tracking::domain::base_url::BaseUrl;

pub fn project_url(base_url: &BaseUrl, project_id: u64) -> String {
    format!("{}/app/projects/{project_id}", base_url.as_str())
}

pub fn tasklist_url(base_url: &BaseUrl, tasklist_id: u64) -> String {
    format!("{}/app/tasklists/{tasklist_id}", base_url.as_str())
}

pub fn task_url(base_url: &BaseUrl, task_id: u64) -> String {
    format!("{}/app/tasks/{task_id}", base_url.as_str())
}
