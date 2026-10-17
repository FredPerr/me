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

/// Where a pull-request suggestion runs and what it is about: everything
/// `suggest_pull_request` needs from the workspace to build the prompt and pick
/// the folder, without `ai_assist` importing `workspace`. A composition adapter
/// implements it over `WorkspaceService`, resolving the values exactly as the
/// create-pull-request modal does today.
pub trait PullRequestContext: Send + Sync {
    /// Resolves the context's checkout of `repository_id` under `context_id` in
    /// `project_tag`: the context and repository names, the worktree path to
    /// run in, and the branch the diff is taken against. Errors when the
    /// project, context or repository is unknown, or the repository has no base
    /// branch to compare against.
    fn resolve(
        &self,
        project_tag: &str,
        context_id: &str,
        repository_id: &str,
    ) -> Result<PullRequestTarget, PullRequestContextError>;
}

/// The resolved target of a pull-request suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestTarget {
    /// Human-readable name of the context (the title fallback).
    pub context_name: String,
    /// Name of the repository the pull request targets.
    pub repository_name: String,
    /// The context's checkout of the repository; the run's working directory.
    pub worktree_path: String,
    /// Branch the diff is computed against (the pull request's base).
    pub base_branch: String,
}

/// Why a pull-request target could not be resolved. Each maps to a distinct
/// thing the user is told; `suggest_pull_request` forwards it to its own error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullRequestContextError {
    /// The project, context or repository could not be found.
    NotFound { reason: String },
    /// The repository has no base branch to compare the changes against.
    NoBaseBranch,
}

/// Reads a worktree's diff against a base branch and its pull request template,
/// so `suggest_pull_request` gets the same diff and template the frontend read
/// through `getWorktreeDiff`/`readPullRequestTemplate` — without `ai_assist`
/// importing `git`. A composition adapter implements it over `GitRepositories`
/// and the pull-request-template reader.
#[async_trait]
pub trait RepositoryDiff: Send + Sync {
    /// The unified diff of the worktree at `worktree_path` against
    /// `base_branch`, and the repository's pull request template if it has one.
    async fn read(
        &self,
        worktree_path: &str,
        base_branch: &str,
    ) -> Result<RepositoryDiffContents, RepositoryDiffUnavailable>;
}

/// A worktree's diff against its base and its optional pull request template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryDiffContents {
    /// Unified diff of the branch against the base. May be empty (no changes).
    pub diff: String,
    /// The repository's pull request template, if one exists.
    pub template: Option<String>,
}

/// The diff or template could not be read (the worktree is missing, the base
/// branch is unknown, git failed). The run cannot proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryDiffUnavailable {
    pub reason: String,
}
