//! Adapters joining bounded contexts that must not know each other. Only the
//! application setup names them.

pub mod ai_session_agent_runner;
pub mod ai_session_ai_assist_runner;
pub mod git_hosting_push_listener;
pub mod git_worktree_pushes;
pub mod settings_default_ai_tool;
pub mod tempfile_temp_workspace;
pub mod workspace_project_branch_naming;
pub mod workspace_project_remotes;
pub mod workspace_project_task_links;
