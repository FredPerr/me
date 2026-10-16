//! Bitbucket Cloud's JSON answers, as sent.

use serde::Deserialize;

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
}

#[derive(Debug, Deserialize)]
pub struct PullRequestBranch {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestLinks {
    pub html: Option<Link>,
}
