//! Adapters of the Workspace ports: the redb project repository for the app,
//! an in-memory one for tests, and the temporary import of the old JSON files.
#[cfg(test)]
pub mod in_memory_project_repository;
pub mod legacy_project_import;
#[cfg(test)]
mod project_repository_contract;
pub mod redb_project_repository;
