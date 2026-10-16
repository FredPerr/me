//! Use cases of the Git Hosting context: signing in and out, the accounts,
//! the open pull requests of a Project, the pull request of each context
//! branch and creating a pull request.

mod branch_pull_request_cache;
#[cfg(test)]
mod context_branch_pull_requests_tests;
#[cfg(test)]
mod create_pull_request_tests;
#[cfg(test)]
mod project_pull_requests_tests;
pub mod registry;
pub mod service;
#[cfg(test)]
mod service_tests;
#[cfg(test)]
pub mod test_support;
