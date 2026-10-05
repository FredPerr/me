//! HTTP client for Teamwork API v3 (reads via `GET`, task mutations via
//! `PATCH`). Errors never include the URL, headers or key.
use std::time::Duration;

use reqwest::header::{ACCEPT, RETRY_AFTER};
use reqwest::Url;
use serde::de::DeserializeOwned;

use crate::work_tracking::domain::api_key::ApiKey;
use crate::work_tracking::domain::base_url::BaseUrl;
use crate::work_tracking::domain::error::{InputField, InvalidInputReason, WorkTrackingError};

const USER_AGENT: &str = "me-app";
const CONNECT_TIMEOUT_SECONDS: u64 = 10;
const REQUEST_TIMEOUT_SECONDS: u64 = 30;
const MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;
/// Teamwork Basic auth takes the API key as user name and any password.
const BASIC_AUTH_PASSWORD: &str = "X";

pub const ME_PATH: &[&str] = &["projects", "api", "v3", "me.json"];
pub const PROJECTS_PATH: &[&str] = &["projects", "api", "v3", "projects.json"];

/// `PATCH /projects/api/v3/tasks/{id}.json`. The id is a validated numeric
/// string, so the composed `{id}.json` segment is always safe.
pub fn task_path(task_id: &str) -> [String; 5] {
    [
        "projects".to_string(),
        "api".to_string(),
        "v3".to_string(),
        "tasks".to_string(),
        format!("{task_id}.json"),
    ]
}

pub const PROJECTS_QUERY: &[(&str, &str)] = &[("includeArchivedProjects", "false")];
pub const TASKLISTS_QUERY: &[(&str, &str)] = &[("showCompleted", "false")];
pub const TASKS_QUERY: &[(&str, &str)] = &[
    ("include", "users,tags"),
    ("getSubTasks", "true"),
    ("nestSubTasks", "false"),
    ("includeCompletedTasks", "false"),
];

pub fn tasklists_path(project_id: &str) -> [&str; 6] {
    [
        "projects",
        "api",
        "v3",
        "projects",
        project_id,
        "tasklists.json",
    ]
}

pub fn tasks_path(tasklist_id: &str) -> [&str; 6] {
    [
        "projects",
        "api",
        "v3",
        "tasklists",
        tasklist_id,
        "tasks.json",
    ]
}

pub fn project_tasks_path(project_id: &str) -> [&str; 6] {
    [
        "projects",
        "api",
        "v3",
        "projects",
        project_id,
        "tasks.json",
    ]
}

/// Explicit filters followed by `page` and `pageSize`.
pub fn paged_query(
    filters: &[(&'static str, &'static str)],
    page: u32,
    page_size: u32,
) -> Vec<(&'static str, String)> {
    filters
        .iter()
        .map(|(name, value)| (*name, value.to_string()))
        .chain([
            ("page", page.to_string()),
            ("pageSize", page_size.to_string()),
        ])
        .collect()
}

/// Segments are percent-encoded one by one, so the host is always the base host.
fn build_url<S: AsRef<str>>(base: &Url, path_segments: &[S], query: &[(&str, String)]) -> Url {
    let mut url = base.clone();
    if let Ok(mut segments) = url.path_segments_mut() {
        segments.pop_if_empty();
        for segment in path_segments {
            segments.push(segment.as_ref());
        }
    }
    if !query.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (name, value) in query {
            pairs.append_pair(name, value);
        }
    }
    url
}

fn map_status(status: u16, retry_after: Option<&str>) -> Result<(), WorkTrackingError> {
    match status {
        200..=299 => Ok(()),
        401 => Err(WorkTrackingError::Unauthorized),
        403 => Err(WorkTrackingError::Forbidden),
        404 => Err(WorkTrackingError::NotFound),
        429 => Err(WorkTrackingError::RateLimited {
            retry_after_seconds: retry_after.and_then(|value| value.trim().parse().ok()),
        }),
        _ => Err(WorkTrackingError::ProviderError {
            status: Some(status),
        }),
    }
}

fn exceeds_limit(current: usize, next: usize) -> bool {
    current.saturating_add(next) > MAX_RESPONSE_BYTES
}

/// Used for both send and body-read failures.
fn map_body_error(error: reqwest::Error) -> WorkTrackingError {
    if error.is_timeout() {
        WorkTrackingError::Timeout
    } else {
        WorkTrackingError::Network
    }
}

pub struct TeamworkClient {
    http: reqwest::Client,
    base_url: Url,
    api_key: ApiKey,
}

impl TeamworkClient {
    pub fn new(base_url: &BaseUrl, api_key: ApiKey) -> Result<Self, WorkTrackingError> {
        // Cannot fail for a validated BaseUrl; mapped defensively.
        let base_url = Url::parse(base_url.as_str()).map_err(|_| {
            WorkTrackingError::invalid_input(InputField::BaseUrl, InvalidInputReason::InvalidFormat)
        })?;
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .https_only(true)
            // Never forward the Basic credential to another location.
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .build()
            .map_err(|_| WorkTrackingError::Network)?;
        Ok(Self {
            http,
            base_url,
            api_key,
        })
    }

    /// Reads JSON from a `GET` endpoint.
    pub async fn get_json<T: DeserializeOwned, S: AsRef<str>>(
        &self,
        path_segments: &[S],
        query: &[(&str, String)],
    ) -> Result<T, WorkTrackingError> {
        let url = build_url(&self.base_url, path_segments, query);
        let response = self
            .http
            .get(url)
            .basic_auth(self.api_key.expose(), Some(BASIC_AUTH_PASSWORD))
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(map_body_error)?;
        self.read_json(response).await
    }

    /// Sends a `PATCH` with a JSON body and parses the JSON response. Used for
    /// task mutations (e.g. assigning a user).
    pub async fn patch_json<T: DeserializeOwned, S: AsRef<str>>(
        &self,
        path_segments: &[S],
        body: &serde_json::Value,
    ) -> Result<T, WorkTrackingError> {
        let url = build_url(&self.base_url, path_segments, &[]);
        let response = self
            .http
            .patch(url)
            .basic_auth(self.api_key.expose(), Some(BASIC_AUTH_PASSWORD))
            .header(ACCEPT, "application/json")
            .json(body)
            .send()
            .await
            .map_err(map_body_error)?;
        self.read_json(response).await
    }

    /// Validates the status and reads a size-bounded JSON body. Shared by the
    /// read and write request methods.
    async fn read_json<T: DeserializeOwned>(
        &self,
        mut response: reqwest::Response,
    ) -> Result<T, WorkTrackingError> {
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        map_status(response.status().as_u16(), retry_after.as_deref())?;

        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(WorkTrackingError::InvalidResponse);
        }
        // Read incrementally so chunked bodies are bounded too.
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(map_body_error)? {
            if exceeds_limit(body.len(), chunk.len()) {
                return Err(WorkTrackingError::InvalidResponse);
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body).map_err(|_| WorkTrackingError::InvalidResponse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://acme.teamwork.com").unwrap()
    }

    fn decoded_query(url: &Url) -> Vec<(String, String)> {
        url.query_pairs()
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect()
    }

    fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
        expected
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn map_status_cases() {
        assert_eq!(map_status(200, None), Ok(()));
        assert_eq!(map_status(204, None), Ok(()));
        assert_eq!(
            map_status(301, None),
            Err(WorkTrackingError::ProviderError { status: Some(301) })
        );
        assert_eq!(map_status(401, None), Err(WorkTrackingError::Unauthorized));
        assert_eq!(map_status(403, None), Err(WorkTrackingError::Forbidden));
        assert_eq!(map_status(404, None), Err(WorkTrackingError::NotFound));
        assert_eq!(
            map_status(429, Some(" 30 ")),
            Err(WorkTrackingError::RateLimited {
                retry_after_seconds: Some(30)
            })
        );
        assert_eq!(
            map_status(429, None),
            Err(WorkTrackingError::RateLimited {
                retry_after_seconds: None
            })
        );
        assert_eq!(
            map_status(429, Some("Wed, 21 Oct 2015 07:28:00 GMT")),
            Err(WorkTrackingError::RateLimited {
                retry_after_seconds: None
            })
        );
        assert_eq!(
            map_status(500, None),
            Err(WorkTrackingError::ProviderError { status: Some(500) })
        );
        assert_eq!(
            map_status(503, None),
            Err(WorkTrackingError::ProviderError { status: Some(503) })
        );
    }

    #[test]
    fn build_url_encodes_segments_and_keeps_host() {
        let url = build_url(&base(), &["projects", "a/b?c#d", "..", "x.json"], &[]);
        assert_eq!(url.host_str(), Some("acme.teamwork.com"));
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.query(), None);
        // Dot segments are dropped by `push`, so a segment can never traverse.
        assert_eq!(url.path(), "/projects/a%2Fb%3Fc%23d/x.json");

        let custom_port = Url::parse("https://acme.teamwork.com:8443").unwrap();
        let url = build_url(&custom_port, ME_PATH, &[]);
        assert_eq!(
            url.as_str(),
            "https://acme.teamwork.com:8443/projects/api/v3/me.json"
        );
    }

    #[test]
    fn build_url_for_projects() {
        let url = build_url(&base(), PROJECTS_PATH, &paged_query(PROJECTS_QUERY, 1, 50));
        assert_eq!(url.path(), "/projects/api/v3/projects.json");
        assert_eq!(
            decoded_query(&url),
            pairs(&[
                ("includeArchivedProjects", "false"),
                ("page", "1"),
                ("pageSize", "50")
            ])
        );
    }

    #[test]
    fn build_url_for_tasklists() {
        let url = build_url(
            &base(),
            &tasklists_path("42"),
            &paged_query(TASKLISTS_QUERY, 2, 50),
        );
        assert_eq!(url.path(), "/projects/api/v3/projects/42/tasklists.json");
        assert_eq!(
            decoded_query(&url),
            pairs(&[
                ("showCompleted", "false"),
                ("page", "2"),
                ("pageSize", "50")
            ])
        );
    }

    #[test]
    fn build_url_for_tasks() {
        let url = build_url(&base(), &tasks_path("7"), &paged_query(TASKS_QUERY, 3, 100));
        assert_eq!(url.path(), "/projects/api/v3/tasklists/7/tasks.json");
        assert_eq!(
            decoded_query(&url),
            pairs(&[
                ("include", "users,tags"),
                ("getSubTasks", "true"),
                ("nestSubTasks", "false"),
                ("includeCompletedTasks", "false"),
                ("page", "3"),
                ("pageSize", "100"),
            ])
        );
    }

    #[test]
    fn build_url_for_project_tasks() {
        let url = build_url(
            &base(),
            &project_tasks_path("42"),
            &paged_query(TASKS_QUERY, 1, 50),
        );
        assert_eq!(url.path(), "/projects/api/v3/projects/42/tasks.json");
        assert_eq!(
            decoded_query(&url),
            pairs(&[
                ("include", "users,tags"),
                ("getSubTasks", "true"),
                ("nestSubTasks", "false"),
                ("includeCompletedTasks", "false"),
                ("page", "1"),
                ("pageSize", "50"),
            ])
        );
    }

    #[test]
    fn build_url_for_single_task() {
        let url = build_url(&base(), &task_path("42"), &[]);
        assert_eq!(url.path(), "/projects/api/v3/tasks/42.json");
        assert_eq!(url.query(), None);
    }

    #[test]
    fn exceeds_limit_boundaries() {
        assert!(!exceeds_limit(MAX_RESPONSE_BYTES - 1, 0));
        assert!(!exceeds_limit(MAX_RESPONSE_BYTES - 1, 1));
        assert!(exceeds_limit(MAX_RESPONSE_BYTES, 1));
        assert!(exceeds_limit(usize::MAX, 1));
    }

    #[test]
    fn transport_constants() {
        assert_eq!(CONNECT_TIMEOUT_SECONDS, 10);
        assert_eq!(REQUEST_TIMEOUT_SECONDS, 30);
        assert_eq!(MAX_RESPONSE_BYTES, 10 * 1024 * 1024);
        assert_eq!(USER_AGENT, "me-app");
    }

    #[test]
    fn client_builds_without_network() {
        let base_url = BaseUrl::parse("https://acme.teamwork.com").unwrap();
        let key = ApiKey::new("key".into()).unwrap();
        assert!(TeamworkClient::new(&base_url, key).is_ok());
    }
}
