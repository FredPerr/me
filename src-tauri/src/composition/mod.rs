//! Adapters joining bounded contexts that must not know each other. Only the
//! application setup names them.

pub mod ai_session_agent_runner;
pub mod git_hosting_push_listener;
pub mod git_worktree_pushes;
pub mod workspace_project_remotes;
pub mod workspace_project_task_links;
