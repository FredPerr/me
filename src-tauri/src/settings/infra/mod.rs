//! Adapters of the Settings ports: the redb settings store, with an in-memory
//! one for tests.

#[cfg(test)]
pub mod in_memory_settings_store;
pub mod redb_settings_store;
#[cfg(test)]
mod settings_store_contract;
