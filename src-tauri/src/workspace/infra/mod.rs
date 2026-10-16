//! Adapters of the Workspace ports: the redb project repository for the app,
//! an in-memory one for tests, and the temporary import of the old JSON files;
//! the checkout files on disk and the `project-changed` event, with recording
//! fakes for tests.
pub mod file_system_checkout_files;
#[cfg(test)]
pub mod in_memory_project_repository;
pub mod legacy_project_import;
#[cfg(test)]
mod project_repository_contract;
#[cfg(test)]
pub mod recording_fakes;
pub mod redb_project_repository;
pub mod tauri_project_events;
