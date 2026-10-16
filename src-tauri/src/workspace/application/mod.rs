//! Use cases of the Workspace context: each loads a Project, does the git and
//! file work, and saves through the Project aggregate. Depends only on
//! `domain` and the git module's ports.
mod checkout_journal;
pub mod error;
pub mod service;
