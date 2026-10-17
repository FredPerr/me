//! Bitbucket Cloud's JSON requests and answers, as sent.

use serde::{Deserialize, Serialize};

/// `POST https://api.bitbucket.org/2.0/repositories/{workspace}/{slug}/pullrequests`
#[derive(Debug, Serialize)]
pub struct CreatePullRequestRequest<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub source: BranchEndpointRequest<'a>,
    pub destination: BranchEndpointRequest<'a>,
    pub close_source_branch: bool,
    pub draft: bool,
}

#[derive(Debug, Serialize)]
pub struct BranchEndpointRequest<'a> {
    pub branch: BranchNameRequest<'a>,
}

#[derive(Debug, Serialize)]
pub struct BranchNameRequest<'a> {
    pub name: &'a str,
}

/// The answer to [`CreatePullRequestRequest`].
#[derive(Debug, Deserialize)]
pub struct CreatedPullRequestResponse {
    pub id: u32,
    pub links: Option<PullRequestLinks>,
}

/// `GET https://api.bitbucket.org/2.0/user`
#[derive(Debug, Deserialize)]
pub struct UserResponse {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub links: Option<UserLinks>,
}

#[derive(Debug, Deserialize)]
pub struct UserLinks {
    pub avatar: Option<Link>,
}

#[derive(Debug, Deserialize)]
pub struct Link {
    pub href: Option<String>,
}

/// `GET https://api.bitbucket.org/2.0/repositories/{workspace}/{slug}/pullrequests`
#[derive(Debug, Deserialize)]
pub struct PullRequestPage {
    #[serde(default)]
    pub values: Vec<PullRequestResponse>,
    pub next: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestResponse {
    pub id: u32,
    pub title: String,
    pub author: Option<PullRequestAuthor>,
    pub source: PullRequestEndpoint,
    pub destination: PullRequestEndpoint,
    #[serde(default)]
    pub updated_on: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub state: String,
    pub links: Option<PullRequestLinks>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestAuthor {
    pub nickname: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestEndpoint {
    pub branch: PullRequestBranch,
    /// Absent on the list endpoint when the source fork is gone.
    pub repository: Option<PullRequestRepository>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestBranch {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestRepository {
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestLinks {
    pub html: Option<Link>,
}
