//! The Project aggregate and its value types. Depends only on `std`, `serde`,
//! `uuid`, `unicode-normalization` and the git module's `BranchName`.

pub mod agent_work;
pub mod branch_naming;
pub mod context;
pub mod context_branch;
pub mod context_folder;
pub mod context_name;
pub mod context_origin;
pub mod context_status;
pub mod default_branch;
pub mod error;
pub mod identifiers;
pub mod ports;
pub mod project;
pub mod project_paths;
pub mod project_tag;
pub mod remote_project_link;
pub mod repository;
pub mod slug;

#[cfg(test)]
pub mod test_support;
