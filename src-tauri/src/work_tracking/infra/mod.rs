//! Adapters of the Work Tracking ports: keychain credentials, the redb
//! connection repository (with a one-time import from the old JSON file) and
//! the provider adapters.
pub mod connection_import;
#[cfg(test)]
mod connection_repository_contract;
#[cfg(test)]
pub mod in_memory_connection_repository;
pub mod keychain_credential_store;
pub mod redb_connection_repository;
pub mod teamwork;
