//! The ports `suggest_contexts` reaches the rest of the app through. The
//! `ai_assist` context owns no Projects, no settings and no disk; it reads what
//! it needs here, and composition adapters implement each port over the owning
//! context (settings, workspace) or the file system.

use async_trait::async_trait;

/// Names a draft's branch under a project's branch naming policy, without
/// `ai_assist` importing `workspace`. A composition adapter implements it over
/// `WorkspaceService`, reusing the same `BranchNamingPolicy` the
/// `workspace_suggest_branch_name` command uses — so a drafted branch is named
/// exactly as a hand-typed one would be.
pub trait ProjectBranchNaming: Send + Sync {
    /// The valid branch name for `proposed_name` under `project_tag`'s policy
    /// (the policy's default prefix is applied), or an error when the name
    /// cannot be made valid (empty slug, unknown project). The use case drops a
    /// draft whose branch errors here.
    fn name_branch(
        &self,
        project_tag: &str,
        proposed_name: &str,
    ) -> Result<String, BranchNamingUnavailable>;
}

/// A branch name could not be produced from the proposed name under the
/// project's policy, or the project could not be read. The reason is not
/// modeled: the caller only needs to know the draft cannot be named, so it is
/// dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchNamingUnavailable;

/// A fresh, empty directory an AI assist runs in so it can touch no worktree,
/// created and cleaned up around the run. A composition adapter implements it
/// with `tempfile`; the use case holds the port so it is testable without disk.
#[async_trait]
pub trait TempWorkspace: Send + Sync {
    /// Creates a fresh empty directory and returns a handle whose `path` is the
    /// directory to run in. Dropping the handle removes the directory, so the
    /// run leaves nothing behind.
    async fn create(&self) -> Result<Box<dyn TempDirectory>, TempWorkspaceUnavailable>;
}

/// A created temporary directory. The directory lives for as long as the handle
/// does; dropping it cleans the directory up.
pub trait TempDirectory: Send + Sync {
    /// The absolute path of the temporary directory to run in.
    fn path(&self) -> String;
}

/// A fresh temporary directory could not be created. The run cannot proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempWorkspaceUnavailable {
    pub reason: String,
}

/// Reads which AI tool the app runs by default, as the adapter id the runner
/// spawns (e.g. `"kiro"`). A composition adapter implements it over the
/// settings service, mapping the `AiTool` enum to its adapter id; `ai_assist`
/// never imports `settings`.
pub trait DefaultAiTool: Send + Sync {
    /// The adapter id of the default AI tool, or the fixed Kiro id when the
    /// settings cannot be read (the tool is always available).
    fn tool_id(&self) -> String;
}
