use async_trait::async_trait;

use super::client::{BitbucketClient, USER_URL};
use super::mapping;
use super::wire::UserResponse;
use crate::git_hosting::domain::error::GitHostingError;
use crate::git_hosting::domain::hosted_account::HostedAccount;
use crate::git_hosting::domain::ports::GitHost;

pub struct BitbucketHost {
    client: BitbucketClient,
    username: String,
}

impl BitbucketHost {
    pub fn new(client: BitbucketClient, username: &str) -> Self {
        Self {
            client,
            username: username.to_string(),
        }
    }
}

#[async_trait]
impl GitHost for BitbucketHost {
    async fn current_account(&self) -> Result<HostedAccount, GitHostingError> {
        let user: UserResponse = self.client.get_json(USER_URL, "current_account").await?;
        Ok(mapping::account(user, &self.username))
    }
}
