//! Adapters of the Git Hosting ports: the GitHub and Bitbucket APIs, keychain
//! secrets, the redb account store (with an in-memory one for tests), the
//! system clock, the `git-hosting-changed` event and the credentials handed to
//! the `git` module.

mod base64;
pub mod bitbucket;
pub mod github;
pub mod host_token_credentials;
#[cfg(test)]
mod hosted_accounts_contract;
mod http;
#[cfg(test)]
pub mod in_memory_hosted_accounts;
pub mod keychain_host_tokens;
mod paging;
pub mod redb_hosted_accounts;
pub mod system_clock;
pub mod tauri_git_hosting_events;
