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
