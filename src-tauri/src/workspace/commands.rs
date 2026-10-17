//! Typed Tauri commands of the Workspace context and their wire types. The
//! Project goes out with every path the frontend needs already computed.
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::application::agent_runs::AgentRunService;
use super::application::error::{AgentRunServiceError, WorkspaceServiceError};
use super::application::service::{
    BranchChoice, CreateContextCommand, RepositoryBranchChoice, WorkspaceService,
};
use super::domain::agent_run::{AgentRun, AgentRunState};
use super::domain::agent_work::{AgentTool, AgentWork};
use super::domain::branch_naming::{parse_branch_name, BranchNamingPolicy};
use super::domain::context::Context;
use super::domain::context_branch::PullRequestDraft;
use super::domain::context_branch::{ContextBranch, ContextCheckout};
use super::domain::context_name::ContextName;
use super::domain::context_origin::{ContextOrigin, TaskRef};
use super::domain::context_status::ContextStatus;
use super::domain::error::WorkspaceError;
use super::domain::identifiers::{ContextId, RepositoryId};
use super::domain::ports::ProjectListing;
use super::domain::project::{Project, ProjectSettings};
use super::domain::project_tag::ProjectTag;
use super::domain::remote_project_link::RemoteProjectLink;
use super::domain::repository::Repository;
use super::Workspace;
use crate::git::domain::error::GitError;

/// The kind of a workspace failure, as the frontend sees it. `invalidInput`
/// is a blank or malformed name, id, branch name or prefix; `internal` is a
/// command's background task failing unexpectedly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceErrorKind {
    ProjectNotFound,
    ProjectAlreadyExists,
    InvalidInput,
    DuplicateContextName,
    DuplicateContextFolder,
    UnknownContext,
    UnknownRepository,
    DefaultContextIsFixed,
    StaticContextStatusLocked,
    LinkedRepositoryIsReadOnly,
    BranchAlreadyExists,
    BranchCheckedOut,
    Git,
    Files,
    Storage,
    /// The agent tool is unknown, the prompt empty, or the run could not be
    /// started or stopped.
    AgentRun,
    /// No agent is running for the context, so there is none to stop.
    NoRunningAgent,
    Internal,
}

/// A failed workspace command on the wire. `message` is readable and may name
/// the user's own projects, contexts, repositories and branches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCommandError {
    pub kind: WorkspaceErrorKind,
    pub message: String,
}

impl WorkspaceCommandError {
    fn task_failed(detail: impl std::fmt::Display) -> Self {
        Self {
            kind: WorkspaceErrorKind::Internal,
            message: format!("The workspace task failed: {detail}"),
        }
    }
}

fn workspace_error_kind(error: &WorkspaceError) -> WorkspaceErrorKind {
    match error {
        WorkspaceError::EmptyProjectTag
        | WorkspaceError::EmptyProjectName
        | WorkspaceError::EmptyProjectPath
        | WorkspaceError::DuplicateRepository { .. }
        | WorkspaceError::EmptyIdentifier
        | WorkspaceError::EmptyContextName
        | WorkspaceError::EmptyContextFolder { .. }
        | WorkspaceError::DuplicateContextBranch { .. }
        | WorkspaceError::InvalidBranchName { .. }
        | WorkspaceError::EmptyBranchName { .. }
        | WorkspaceError::InvalidPrefix { .. }
        | WorkspaceError::UnknownPrefix { .. }
        | WorkspaceError::PrefixRequired => WorkspaceErrorKind::InvalidInput,
        WorkspaceError::DuplicateContextName { .. } => WorkspaceErrorKind::DuplicateContextName,
        WorkspaceError::DuplicateContextFolder { .. } => WorkspaceErrorKind::DuplicateContextFolder,
        WorkspaceError::UnknownContext { .. }
        | WorkspaceError::UnknownBaseContext { .. }
        | WorkspaceError::UnknownContextBranch { .. } => WorkspaceErrorKind::UnknownContext,
        WorkspaceError::UnknownRepository { .. } => WorkspaceErrorKind::UnknownRepository,
        WorkspaceError::DefaultContextAlreadyExists | WorkspaceError::DefaultContextIsFixed => {
            WorkspaceErrorKind::DefaultContextIsFixed
        }
        WorkspaceError::StaticContextStatusLocked { .. } => {
            WorkspaceErrorKind::StaticContextStatusLocked
        }
        WorkspaceError::LinkedRepositoryIsReadOnly { .. } => {
            WorkspaceErrorKind::LinkedRepositoryIsReadOnly
        }
    }
}

impl From<WorkspaceError> for WorkspaceCommandError {
    fn from(error: WorkspaceError) -> Self {
        Self {
            kind: workspace_error_kind(&error),
            message: error.to_string(),
        }
    }
}

impl From<WorkspaceServiceError> for WorkspaceCommandError {
    fn from(error: WorkspaceServiceError) -> Self {
        let kind = match &error {
            WorkspaceServiceError::ProjectNotFound { .. } => WorkspaceErrorKind::ProjectNotFound,
            WorkspaceServiceError::ProjectAlreadyExists { .. } => {
                WorkspaceErrorKind::ProjectAlreadyExists
            }
            WorkspaceServiceError::Rejected(rejection) => workspace_error_kind(rejection),
            WorkspaceServiceError::Storage(_) => WorkspaceErrorKind::Storage,
            WorkspaceServiceError::NewBranchAlreadyExists { .. }
            | WorkspaceServiceError::Git {
                error: GitError::BranchAlreadyExists { .. },
                ..
            } => WorkspaceErrorKind::BranchAlreadyExists,
            WorkspaceServiceError::BranchCheckedOut { .. }
            | WorkspaceServiceError::Git {
                error: GitError::BranchCheckedOut { .. },
                ..
            } => WorkspaceErrorKind::BranchCheckedOut,
            WorkspaceServiceError::Git { .. } => WorkspaceErrorKind::Git,
            WorkspaceServiceError::Files(_) => WorkspaceErrorKind::Files,
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}

impl From<AgentRunServiceError> for WorkspaceCommandError {
    fn from(error: AgentRunServiceError) -> Self {
        match error {
            AgentRunServiceError::Workspace(workspace) => workspace.into(),
            AgentRunServiceError::Runner(_) => Self {
                kind: WorkspaceErrorKind::AgentRun,
                message: error.to_string(),
            },
            AgentRunServiceError::NoRunningAgent { .. } => Self {
                kind: WorkspaceErrorKind::NoRunningAgent,
                message: error.to_string(),
            },
        }
    }
}

/// How one of a context's agent runs looks to the frontend: its transcript
/// (oldest first), how it ended and when it started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunView {
    pub id: String,
    pub context_id: String,
    pub prompt: String,
    pub output_lines: Vec<String>,
    pub state: AgentRunStateView,
    pub exit_code: Option<i32>,
    pub trimmed: bool,
    pub started_at: String,
}

/// How a run ended, or that it is still going, on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum AgentRunStateView {
    Running,
    Finished,
    Failed,
    Stopped,
    Interrupted,
}

impl From<AgentRunState> for AgentRunStateView {
    fn from(state: AgentRunState) -> Self {
        match state {
            AgentRunState::Running => Self::Running,
            AgentRunState::Finished => Self::Finished,
            AgentRunState::Failed => Self::Failed,
            AgentRunState::Stopped => Self::Stopped,
            AgentRunState::Interrupted => Self::Interrupted,
        }
    }
}

impl From<AgentRun> for AgentRunView {
    fn from(run: AgentRun) -> Self {
        Self {
            id: run.id.to_string(),
            context_id: run.context_id.to_string(),
            prompt: run.prompt,
            output_lines: run.output_lines,
            state: run.state.into(),
            exit_code: run.exit_code,
            trimmed: run.trimmed,
            started_at: run.started_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryView {
    pub id: String,
    pub name: String,
    pub relative_path: String,
    pub post_checkout_command: Option<String>,
    /// The repository's folder: its main checkout.
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ContextStatusView {
    Idle,
    Ready,
    InProgress,
    NeedsHuman,
    Review,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ContextOriginView {
    Manual,
    FromContext,
    FromBranches,
    FromPullRequests,
    #[serde(rename_all = "camelCase")]
    FromTask {
        connection_id: String,
        task_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "tool", rename_all = "camelCase")]
pub enum AgentToolView {
    #[serde(rename_all = "camelCase")]
    Kiro { conversation_id: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkView {
    pub prompt: Option<String>,
    pub tool: AgentToolView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestDraftView {
    pub title: String,
    pub description: String,
}

/// `linked` is a legacy linked repository: a symlink to another checkout,
/// which can only be deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ContextCheckoutView {
    Worktree,
    Linked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContextBranchView {
    pub repository_id: String,
    pub branch: String,
    pub checkout: ContextCheckoutView,
    /// `null` when the repository is no longer part of the project.
    pub worktree_path: Option<String>,
    pub pull_request_draft: Option<PullRequestDraftView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContextView {
    pub id: String,
    pub name: String,
    pub folder: String,
    pub folder_path: String,
    pub ide_path: String,
    pub status: ContextStatusView,
    pub is_default: bool,
    pub is_static: bool,
    pub base_context_id: Option<String>,
    pub branches: Vec<ContextBranchView>,
    pub origin: ContextOriginView,
    pub agent_work: Option<AgentWorkView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BranchNamingView {
    pub prefixes: Vec<String>,
    pub allows_no_prefix: bool,
    pub default_prefix: Option<String>,
}

/// Also the input shape: links are saved as the frontend sends them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteProjectLinkView {
    pub provider_kind: String,
    pub connection_id: String,
    pub remote_project_id: String,
    pub remote_project_name: String,
    pub remote_project_url: Option<String>,
    pub linked_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub tag: String,
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
    /// As configured; empty for a single-repository project.
    pub repositories: Vec<RepositoryView>,
    /// The repositories contexts check out: the configured ones, or the single
    /// repository at the project root.
    pub effective_repositories: Vec<RepositoryView>,
    pub contexts: Vec<ContextView>,
    pub symlinks: Vec<String>,
    pub remote_project_links: Vec<RemoteProjectLinkView>,
    pub branch_naming: BranchNamingView,
    /// The folder to open the project itself in.
    pub root_ide_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UnreadableProjectView {
    pub tag: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListView {
    pub projects: Vec<ProjectView>,
    /// Stored Projects that couldn't be read.
    pub unreadable_projects: Vec<UnreadableProjectView>,
}

impl From<ContextStatus> for ContextStatusView {
    fn from(status: ContextStatus) -> Self {
        match status {
            ContextStatus::Idle => Self::Idle,
            ContextStatus::Ready => Self::Ready,
            ContextStatus::InProgress => Self::InProgress,
            ContextStatus::NeedsHuman => Self::NeedsHuman,
            ContextStatus::Review => Self::Review,
            ContextStatus::Done => Self::Done,
        }
    }
}

impl From<ContextStatusView> for ContextStatus {
    fn from(status: ContextStatusView) -> Self {
        match status {
            ContextStatusView::Idle => Self::Idle,
            ContextStatusView::Ready => Self::Ready,
            ContextStatusView::InProgress => Self::InProgress,
            ContextStatusView::NeedsHuman => Self::NeedsHuman,
            ContextStatusView::Review => Self::Review,
            ContextStatusView::Done => Self::Done,
        }
    }
}

impl From<&ContextOrigin> for ContextOriginView {
    fn from(origin: &ContextOrigin) -> Self {
        match origin {
            ContextOrigin::Manual => Self::Manual,
            ContextOrigin::FromContext => Self::FromContext,
            ContextOrigin::FromBranches => Self::FromBranches,
            ContextOrigin::FromPullRequests => Self::FromPullRequests,
            ContextOrigin::FromTask(task) => Self::FromTask {
                connection_id: task.connection_id.clone(),
                task_id: task.task_id.clone(),
            },
        }
    }
}

impl From<ContextOriginView> for ContextOrigin {
    fn from(origin: ContextOriginView) -> Self {
        match origin {
            ContextOriginView::Manual => Self::Manual,
            ContextOriginView::FromContext => Self::FromContext,
            ContextOriginView::FromBranches => Self::FromBranches,
            ContextOriginView::FromPullRequests => Self::FromPullRequests,
            ContextOriginView::FromTask {
                connection_id,
                task_id,
            } => Self::FromTask(TaskRef {
                connection_id,
                task_id,
            }),
        }
    }
}

impl From<&AgentWork> for AgentWorkView {
    fn from(agent: &AgentWork) -> Self {
        let tool = match &agent.tool {
            AgentTool::Kiro { conversation_id } => AgentToolView::Kiro {
                conversation_id: conversation_id.clone(),
            },
        };
        Self {
            prompt: agent.prompt.clone(),
            tool,
        }
    }
}

impl From<&BranchNamingPolicy> for BranchNamingView {
    fn from(policy: &BranchNamingPolicy) -> Self {
        Self {
            prefixes: policy.prefixes().iter().map(ToString::to_string).collect(),
            allows_no_prefix: policy.allows_no_prefix(),
            default_prefix: policy.default_prefix().map(ToString::to_string),
        }
    }
}

impl From<&RemoteProjectLink> for RemoteProjectLinkView {
    fn from(link: &RemoteProjectLink) -> Self {
        Self {
            provider_kind: link.provider_kind.clone(),
            connection_id: link.connection_id.clone(),
            remote_project_id: link.remote_project_id.clone(),
            remote_project_name: link.remote_project_name.clone(),
            remote_project_url: link.remote_project_url.clone(),
            linked_at: link.linked_at.clone(),
        }
    }
}

impl RepositoryView {
    fn new(project: &Project, repository: &Repository) -> Self {
        Self {
            id: repository.id.to_string(),
            name: repository.name.clone(),
            relative_path: repository.relative_path.clone(),
            post_checkout_command: repository.post_checkout_command.clone(),
            path: project.repository_path(repository),
        }
    }
}

impl ContextBranchView {
    fn new(project: &Project, context: &Context, branch: &ContextBranch) -> Self {
        Self {
            repository_id: branch.repository.to_string(),
            branch: branch.branch.to_string(),
            checkout: match branch.checkout {
                ContextCheckout::Worktree => ContextCheckoutView::Worktree,
                ContextCheckout::Linked => ContextCheckoutView::Linked,
            },
            worktree_path: project.context_branch_worktree_path(context, branch),
            pull_request_draft: branch
                .pull_request
                .as_ref()
                .map(|link| PullRequestDraftView {
                    title: link.draft.title.clone(),
                    description: link.draft.description.clone(),
                }),
        }
    }
}

impl ContextView {
    fn new(project: &Project, context: &Context) -> Self {
        Self {
            id: context.id.to_string(),
            name: context.name.to_string(),
            folder: context.folder.to_string(),
            folder_path: project.context_folder_path(context),
            ide_path: project.ide_path(context),
            status: context.status.into(),
            is_default: context.is_default,
            is_static: context.is_static,
            base_context_id: context.base.as_ref().map(ToString::to_string),
            branches: context
                .branches
                .iter()
                .map(|branch| ContextBranchView::new(project, context, branch))
                .collect(),
            origin: (&context.origin).into(),
            agent_work: context.agent.as_ref().map(AgentWorkView::from),
        }
    }
}

impl From<&Project> for ProjectView {
    fn from(project: &Project) -> Self {
        let repository_views = |repositories: &[Repository]| {
            repositories
                .iter()
                .map(|repository| RepositoryView::new(project, repository))
                .collect()
        };
        Self {
            tag: project.tag().to_string(),
            name: project.name().to_string(),
            path: project.path().to_string(),
            icon: project.icon().map(str::to_string),
            repositories: repository_views(project.repositories()),
            effective_repositories: repository_views(&project.effective_repositories()),
            contexts: project
                .contexts()
                .iter()
                .map(|context| ContextView::new(project, context))
                .collect(),
            symlinks: project.symlinks().to_vec(),
            remote_project_links: project
                .remote_project_links()
                .iter()
                .map(RemoteProjectLinkView::from)
                .collect(),
            branch_naming: project.branch_naming().into(),
            root_ide_path: project
                .root_ide_path_candidates()
                .into_iter()
                .next()
                .unwrap_or_else(|| project.path().to_string()),
        }
    }
}

impl ProjectListView {
    fn new(listing: ProjectListing) -> Self {
        Self {
            projects: listing.projects.iter().map(ProjectView::from).collect(),
            unreadable_projects: listing
                .unreadable
                .into_iter()
                .map(|record| UnreadableProjectView {
                    tag: record.tag,
                    reason: record.reason,
                })
                .collect(),
        }
    }
}

/// How the new context gets its branch in one repository.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BranchChoiceInput {
    #[serde(rename_all = "camelCase")]
    NewBranch {
        name: String,
        base_branch: String,
    },
    ExistingBranch {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryBranchInput {
    pub repository_id: String,
    pub branch: BranchChoiceInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateContextInput {
    pub project_tag: String,
    pub name: String,
    pub base_context_id: Option<String>,
    pub origin: ContextOriginView,
    /// Ready when `null`.
    pub status: Option<ContextStatusView>,
    pub agent_prompt: Option<String>,
    /// One per repository of the project.
    pub branches: Vec<RepositoryBranchInput>,
}

impl TryFrom<BranchChoiceInput> for BranchChoice {
    type Error = WorkspaceError;

    fn try_from(input: BranchChoiceInput) -> Result<Self, Self::Error> {
        Ok(match input {
            BranchChoiceInput::NewBranch { name, base_branch } => Self::NewBranch {
                name: parse_branch_name(&name)?,
                base_branch: parse_branch_name(&base_branch)?,
            },
            BranchChoiceInput::ExistingBranch { name } => Self::ExistingBranch {
                name: parse_branch_name(&name)?,
            },
        })
    }
}

impl TryFrom<CreateContextInput> for CreateContextCommand {
    type Error = WorkspaceError;

    fn try_from(input: CreateContextInput) -> Result<Self, Self::Error> {
        let branches = input
            .branches
            .into_iter()
            .map(|branch| {
                Ok(RepositoryBranchChoice {
                    repository: RepositoryId::parse(&branch.repository_id)?,
                    choice: branch.branch.try_into()?,
                })
            })
            .collect::<Result<_, WorkspaceError>>()?;
        Ok(Self {
            project: ProjectTag::parse(&input.project_tag)?,
            name: ContextName::parse(&input.name)?,
            base: input
                .base_context_id
                .as_deref()
                .map(ContextId::parse)
                .transpose()?,
            origin: input.origin.into(),
            status: input.status.map(ContextStatus::from),
            agent_prompt: input.agent_prompt,
            branches,
        })
    }
}

impl From<RemoteProjectLinkView> for RemoteProjectLink {
    fn from(link: RemoteProjectLinkView) -> Self {
        Self {
            provider_kind: link.provider_kind,
            connection_id: link.connection_id,
            remote_project_id: link.remote_project_id,
            remote_project_name: link.remote_project_name,
            remote_project_url: link.remote_project_url,
            linked_at: link.linked_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryInput {
    pub id: String,
    pub name: String,
    pub relative_path: String,
    pub post_checkout_command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BranchNamingInput {
    pub prefixes: Vec<String>,
    pub allow_no_prefix: bool,
}

/// Everything the project settings form saves. The tag is not part of it: it
/// never changes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettingsInput {
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
    /// Empty for a single-repository project.
    pub repositories: Vec<RepositoryInput>,
    pub symlinks: Vec<String>,
    pub branch_naming: BranchNamingInput,
    pub remote_project_links: Vec<RemoteProjectLinkView>,
}

impl TryFrom<RepositoryInput> for Repository {
    type Error = WorkspaceError;

    fn try_from(input: RepositoryInput) -> Result<Self, Self::Error> {
        Ok(Self {
            post_checkout_command: input.post_checkout_command,
            ..Self::new(
                RepositoryId::parse(&input.id)?,
                input.name,
                input.relative_path,
            )
        })
    }
}

impl TryFrom<ProjectSettingsInput> for ProjectSettings {
    type Error = WorkspaceError;

    fn try_from(input: ProjectSettingsInput) -> Result<Self, Self::Error> {
        Ok(Self {
            name: input.name,
            path: input.path,
            icon: input.icon,
            repositories: input
                .repositories
                .into_iter()
                .map(Repository::try_from)
                .collect::<Result<_, _>>()?,
            symlinks: input.symlinks,
            remote_project_links: input
                .remote_project_links
                .into_iter()
                .map(RemoteProjectLink::from)
                .collect(),
            branch_naming: BranchNamingPolicy::parse(
                &input.branch_naming.prefixes,
                input.branch_naming.allow_no_prefix,
            )?,
        })
    }
}

/// Runs blocking git and storage work off the async runtime.
async fn run_blocking<T, F>(work: F) -> Result<T, WorkspaceCommandError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, WorkspaceCommandError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(WorkspaceCommandError::task_failed)?
}

fn service(workspace: &State<'_, Workspace>) -> Arc<WorkspaceService> {
    workspace.service.clone()
}

fn agent_run_service(workspace: &State<'_, Workspace>) -> Arc<AgentRunService> {
    workspace.agent_run_service.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_list_projects(
    workspace: State<'_, Workspace>,
) -> Result<ProjectListView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let listing = service.list_projects()?;
        Ok(ProjectListView::new(listing))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_get_project(
    workspace: State<'_, Workspace>,
    tag: String,
) -> Result<ProjectView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&tag)?;
        Ok(ProjectView::from(&service.load_project(&tag)?))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_create_context(
    workspace: State<'_, Workspace>,
    input: CreateContextInput,
) -> Result<ProjectView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let command = CreateContextCommand::try_from(input)?;
        Ok(ProjectView::from(&service.create_context(command)?))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_delete_context(
    workspace: State<'_, Workspace>,
    project_tag: String,
    context_id: String,
) -> Result<ProjectView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&project_tag)?;
        let id = ContextId::parse(&context_id)?;
        Ok(ProjectView::from(&service.delete_context(&tag, &id)?))
    })
    .await
}

/// Static contexts keep their status.
#[tauri::command]
#[specta::specta]
pub async fn workspace_set_context_status(
    workspace: State<'_, Workspace>,
    project_tag: String,
    context_id: String,
    status: ContextStatusView,
) -> Result<ProjectView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&project_tag)?;
        let id = ContextId::parse(&context_id)?;
        let project = service.set_context_status(&tag, &id, status.into())?;
        Ok(ProjectView::from(&project))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_set_context_static(
    workspace: State<'_, Workspace>,
    project_tag: String,
    context_id: String,
    is_static: bool,
) -> Result<ProjectView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&project_tag)?;
        let id = ContextId::parse(&context_id)?;
        let project = service.set_context_static(&tag, &id, is_static)?;
        Ok(ProjectView::from(&project))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_save_pull_request_draft(
    workspace: State<'_, Workspace>,
    project_tag: String,
    context_id: String,
    repository_id: String,
    title: String,
    description: String,
) -> Result<ProjectView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&project_tag)?;
        let id = ContextId::parse(&context_id)?;
        let repository = RepositoryId::parse(&repository_id)?;
        let draft = PullRequestDraft { title, description };
        let project = service.save_pull_request_draft(&tag, &id, &repository, draft)?;
        Ok(ProjectView::from(&project))
    })
    .await
}

/// `null` or a blank prompt clears it.
#[tauri::command]
#[specta::specta]
pub async fn workspace_set_agent_prompt(
    workspace: State<'_, Workspace>,
    project_tag: String,
    context_id: String,
    prompt: Option<String>,
) -> Result<ProjectView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&project_tag)?;
        let id = ContextId::parse(&context_id)?;
        let project = service.set_agent_prompt(&tag, &id, prompt)?;
        Ok(ProjectView::from(&project))
    })
    .await
}

/// Remembers the Kiro conversation so follow-up runs resume it.
#[tauri::command]
#[specta::specta]
pub async fn workspace_record_agent_conversation(
    workspace: State<'_, Workspace>,
    project_tag: String,
    context_id: String,
    conversation_id: String,
) -> Result<ProjectView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&project_tag)?;
        let id = ContextId::parse(&context_id)?;
        let project = service.record_agent_conversation(&tag, &id, conversation_id)?;
        Ok(ProjectView::from(&project))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_update_project_settings(
    workspace: State<'_, Workspace>,
    project_tag: String,
    settings: ProjectSettingsInput,
) -> Result<ProjectView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&project_tag)?;
        let settings = ProjectSettings::try_from(settings)?;
        let project = service.update_project_settings(&tag, settings)?;
        Ok(ProjectView::from(&project))
    })
    .await
}

/// Refuses a tag already in use. The new project gets its default context on
/// each repository's default branch.
#[tauri::command]
#[specta::specta]
pub async fn workspace_add_project(
    workspace: State<'_, Workspace>,
    project_tag: String,
    settings: ProjectSettingsInput,
) -> Result<ProjectView, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&project_tag)?;
        let settings = ProjectSettings::try_from(settings)?;
        Ok(ProjectView::from(&service.add_project(tag, settings)?))
    })
    .await
}

/// Forgets the project; its folders, worktrees and branches stay on disk.
#[tauri::command]
#[specta::specta]
pub async fn workspace_delete_project(
    workspace: State<'_, Workspace>,
    project_tag: String,
) -> Result<(), WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&project_tag)?;
        Ok(service.delete_project(&tag)?)
    })
    .await
}

/// `prefix: null` asks for a name without prefix.
#[tauri::command]
#[specta::specta]
pub async fn workspace_suggest_branch_name(
    workspace: State<'_, Workspace>,
    project_tag: String,
    title: String,
    prefix: Option<String>,
) -> Result<String, WorkspaceCommandError> {
    let service = service(&workspace);
    run_blocking(move || {
        let tag = ProjectTag::parse(&project_tag)?;
        let branch = service.suggest_branch_name(&tag, &title, prefix.as_deref())?;
        Ok(branch.to_string())
    })
    .await
}

/// Starts an agent on the context: Rust picks the folder, seeds the permission
/// floor, records the run and streams an `agent-run` event. A static context
/// keeps its status; others move to in progress, then needs human on end.
#[tauri::command]
#[specta::specta]
pub async fn workspace_run_agent(
    workspace: State<'_, Workspace>,
    project_tag: String,
    context_id: String,
    prompt: String,
) -> Result<(), WorkspaceCommandError> {
    let service = agent_run_service(&workspace);
    let tag = ProjectTag::parse(&project_tag)?;
    let id = ContextId::parse(&context_id)?;
    service.run_agent(&tag, &id, prompt).await?;
    Ok(())
}

/// Stops the context's running agent; its run ends as stopped.
#[tauri::command]
#[specta::specta]
pub async fn workspace_stop_agent_run(
    workspace: State<'_, Workspace>,
    context_id: String,
) -> Result<(), WorkspaceCommandError> {
    let service = agent_run_service(&workspace);
    let id = ContextId::parse(&context_id)?;
    service.stop_agent_run(&id).await?;
    Ok(())
}

/// A context's past runs with their transcripts, oldest first.
#[tauri::command]
#[specta::specta]
pub async fn workspace_list_agent_runs(
    workspace: State<'_, Workspace>,
    context_id: String,
) -> Result<Vec<AgentRunView>, WorkspaceCommandError> {
    let service = agent_run_service(&workspace);
    run_blocking(move || {
        let id = ContextId::parse(&context_id)?;
        let runs = service.list_agent_runs(&id)?;
        Ok(runs.into_iter().map(AgentRunView::from).collect())
    })
    .await
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
