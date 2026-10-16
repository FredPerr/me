//! HTTP plumbing shared by the Git host adapters. Response bodies are only
//! logged, never put in an error.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::HeaderMap;
use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;

use crate::git_hosting::domain::error::GitHostingError;

const USER_AGENT: &str = "me-app";
const CONNECT_TIMEOUT_SECONDS: u64 = 10;
const REQUEST_TIMEOUT_SECONDS: u64 = 30;

/// A Git host's answer, read in full.
pub struct Answer {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Answer {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    pub fn log_failure(&self, host_label: &str, context: &str) {
        eprintln!(
            "[git_hosting] {host_label} {context} failed: status {} body {}",
            self.status,
            String::from_utf8_lossy(&self.body)
        );
    }
}

pub fn client() -> Result<reqwest::Client, GitHostingError> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .https_only(true)
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .build()
        .map_err(|_| GitHostingError::Network)
}

pub async fn send(request: RequestBuilder) -> Result<Answer, GitHostingError> {
    let response = request.send().await.map_err(transport_error)?;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let body = response.bytes().await.map_err(transport_error)?.to_vec();
    Ok(Answer {
        status,
        headers,
        body,
    })
}

pub fn decode<T: DeserializeOwned>(
    answer: &Answer,
    host_label: &str,
    context: &str,
) -> Result<T, GitHostingError> {
    serde_json::from_slice(&answer.body).map_err(|error| {
        eprintln!("[git_hosting] {host_label} {context} unreadable: {error}");
        GitHostingError::InvalidResponse
    })
}

fn transport_error(error: reqwest::Error) -> GitHostingError {
    eprintln!("[git_hosting] request failed: {error}");
    if error.is_timeout() {
        GitHostingError::Timeout
    } else if error.is_decode() {
        GitHostingError::InvalidResponse
    } else {
        GitHostingError::Network
    }
}

/// `Retry-After` in seconds. An HTTP date is not read and gives `None`.
pub fn retry_after_seconds(retry_after: Option<&str>) -> Option<u32> {
    retry_after.and_then(|value| value.trim().parse().ok())
}

pub fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RETRY_AFTER_SECONDS: u32 = 30;

    #[test]
    fn reads_retry_after_in_seconds() {
        assert_eq!(retry_after_seconds(Some(" 30 ")), Some(RETRY_AFTER_SECONDS));
    }

    #[test]
    fn ignores_a_retry_after_date() {
        assert_eq!(
            retry_after_seconds(Some("Wed, 21 Oct 2015 07:28:00 GMT")),
            None
        );
    }

    #[test]
    fn has_no_retry_time_without_the_header() {
        assert_eq!(retry_after_seconds(None), None);
    }
}
