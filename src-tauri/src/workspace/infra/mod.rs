//! Adapters of the Workspace ports: the redb project repository for the app,
//! and an in-memory one for tests.
#[cfg(test)]
pub mod in_memory_project_repository;
#[cfg(test)]
mod project_repository_contract;
pub mod redb_project_repository;
