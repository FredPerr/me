use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use super::checkout_journal::CheckoutJournal;
use super::error::{CheckoutHolder, WorkspaceServiceError};
use crate::git::domain::branch_name::BranchName;
use crate::git::domain::error::GitError;
use crate::git::domain::paths::{RepositoryPath, WorktreePath};
use crate::git::domain::ports::{GitRepositories, GitRepository};
use crate::workspace::domain::agent_work::AgentWork;
use crate::workspace::domain::context::Context;
use crate::workspace::domain::context_branch::{
    ContextBranch, MatchedPullRequest, PullRequestDraft,
};
use crate::workspace::domain::context_name::ContextName;
use crate::workspace::domain::context_origin::ContextOrigin;
use crate::workspace::domain::context_status::ContextStatus;
use crate::workspace::domain::identifiers::{ContextId, RepositoryId};
use crate::workspace::domain::ports::{
    AgentRunRepository, CheckoutFiles, ProjectChange, ProjectEvents, ProjectListing,
    ProjectRepository,
};
use crate::workspace::domain::project::{Project, ProjectSettings};
use crate::workspace::domain::project_tag::ProjectTag;
use crate::workspace::domain::repository::Repository;

/// How a new context gets its branch in one repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchChoice {
    /// A branch created from `base_branch`; refused when `name` already exists.
    NewBranch {
        name: BranchName,
        base_branch: BranchName,
    },
    /// An existing local branch, refused when it is checked out elsewhere.
    ExistingBranch { name: BranchName },
    /// The head branch of a matched pull request. Its head and base are
    /// fetched from `origin` before checkout, and the match is recorded on
    /// the context branch.
    PullRequest {
        head: BranchName,
        base: BranchName,
        matched: MatchedPullRequest,
    },
}

impl BranchChoice {
    /// The branch the context is checked out on in the repository.
    pub fn branch(&self) -> &BranchName {
        match self {
            Self::NewBranch { name, .. }
            | Self::ExistingBranch { name }
            | Self::PullRequest { head: name, .. } => name,
        }
    }

    /// The matched pull request, when the branch comes from one.
    fn matched_pull_request(&self) -> Option<&MatchedPullRequest> {
        match self {
            Self::PullRequest { matched, .. } => Some(matched),
            Self::NewBranch { .. } | Self::ExistingBranch { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryBranchChoice {
    pub repository: RepositoryId,
    pub choice: BranchChoice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateContextCommand {
    pub project: ProjectTag,
    pub name: ContextName,
    pub base: Option<ContextId>,
    pub origin: ContextOrigin,
    /// The status a new context starts in when `None`.
    pub status: Option<ContextStatus>,
    pub agent_prompt: Option<String>,
    pub branches: Vec<RepositoryBranchChoice>,
}

impl CreateContextCommand {
    fn context(&self) -> Result<Context, WorkspaceServiceError> {
        let branches = self
            .branches
            .iter()
            .map(|choice| {
                let repository = choice.repository.clone();
                let branch = choice.choice.branch().clone();
                match choice.choice.matched_pull_request() {
                    Some(matched) => {
                        ContextBranch::from_pull_request(repository, branch, matched.clone())
                    }
                    None => ContextBranch::new(repository, branch),
                }
            })
            .collect();
        let context = Context::new(self.name.clone(), branches)?;
        let agent = AgentWork::default().with_prompt(self.agent_prompt.clone());
        Ok(Context {
            status: self.status.unwrap_or(context.status),
            base: self.base.clone(),
            origin: self.origin.clone(),
            agent: agent.prompt.is_some().then_some(agent),
            ..context
        })
    }
}

/// One repository of a context being created, opened and ready to check out.
struct PlannedCheckout<'a> {
    repository: Repository,
    git: Box<dyn GitRepository>,
    choice: &'a BranchChoice,
    worktree: WorktreePath,
}

/// One repository of a context being deleted.
struct RemovedCheckout<'a> {
    repository: Repository,
    branch: &'a ContextBranch,
    git: Option<Box<dyn GitRepository>>,
}

pub struct WorkspaceService {
    projects: Arc<dyn ProjectRepository>,
    git: Arc<dyn GitRepositories>,
    files: Arc<dyn CheckoutFiles>,
    events: Arc<dyn ProjectEvents>,
    agent_runs: Arc<dyn AgentRunRepository>,
}

impl WorkspaceService {
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        git: Arc<dyn GitRepositories>,
        files: Arc<dyn CheckoutFiles>,
        events: Arc<dyn ProjectEvents>,
        agent_runs: Arc<dyn AgentRunRepository>,
    ) -> Self {
        Self {
            projects,
            git,
            files,
            events,
            agent_runs,
        }
    }

    /// Every readable Project, each with its default context repaired. A
    /// Project whose repair fails is listed as stored.
    pub fn list_projects(&self) -> Result<ProjectListing, WorkspaceServiceError> {
        let listing = self.projects.list()?;
        let projects = listing
            .projects
            .into_iter()
            .map(|project| {
                self.with_repaired_default_context(project.clone())
                    .unwrap_or_else(|error| {
                        eprintln!(
                            "[workspace] default context of '{}' not repaired: {error}",
                            project.tag()
                        );
                        project
                    })
            })
            .collect();
        Ok(ProjectListing {
            projects,
            unreadable: listing.unreadable,
        })
    }

    pub fn load_project(&self, tag: &ProjectTag) -> Result<Project, WorkspaceServiceError> {
        let project = self.find_project(tag)?;
        self.with_repaired_default_context(project)
    }

    /// Checks the context's branches out, one worktree per repository, then
    /// runs the post-checkout commands and links the configured symlinks, and
    /// only then saves the context. Any failure undoes what was done.
    pub fn create_context(
        &self,
        command: CreateContextCommand,
    ) -> Result<Project, WorkspaceServiceError> {
        let project = self.find_project(&command.project)?;
        let context = command.context()?;
        project.clone().add_context(context.clone())?;
        let planned = self.plan_checkouts(&project, &context, &command.branches)?;
        self.fetch_pull_request_branches(&planned)?;

        let mut journal = CheckoutJournal::default();
        let created = self
            .check_out(&project, &context, &planned, &mut journal)
            .and_then(|()| {
                let context = context.clone();
                Ok(self.projects.update(
                    project.tag(),
                    Box::new(move |current| current.add_context(context)),
                )?)
            });
        match created {
            Ok(saved) => {
                self.events.project_changed(saved.tag());
                Ok(saved)
            }
            Err(error) => {
                journal.undo(
                    self.files.as_ref(),
                    Path::new(&project.context_folder_path(&context)),
                );
                Err(error)
            }
        }
    }

    /// Removes the context's worktrees, its configured symlinks and its folder
    /// once empty, deletes the branches nobody needs any more, then removes
    /// the context. Nothing is saved when a worktree can't be removed.
    pub fn delete_context(
        &self,
        tag: &ProjectTag,
        id: &ContextId,
    ) -> Result<Project, WorkspaceServiceError> {
        let project = self.find_project(tag)?;
        project.clone().delete_context(id)?;
        let context = project
            .find_context(id)
            .expect("the Project accepted deleting this context");

        let mut checkouts = Vec::with_capacity(context.branches.len());
        for branch in &context.branches {
            let Some(repository) = project.find_effective_repository(&branch.repository) else {
                continue;
            };
            let repository = repository.into_owned();
            let git = self.remove_checkout(&project, context, &repository, branch)?;
            checkouts.push(RemovedCheckout {
                repository,
                branch,
                git,
            });
        }
        self.remove_context_folder(&project, context);
        for checkout in &checkouts {
            self.delete_branch_if_unused(&project, context, checkout);
        }

        let deleted_id = id.clone();
        let saved = self.projects.update(
            tag,
            Box::new(move |current| current.delete_context(&deleted_id)),
        )?;
        // A context's agent runs go with it. A storage failure here only leaves
        // orphan run records, so it is logged, not surfaced: the context is gone.
        if let Err(error) = self.agent_runs.delete_runs(id) {
            eprintln!("[workspace] agent runs of deleted context '{id}' not removed: {error}");
        }
        self.events.project_changed(saved.tag());
        Ok(saved)
    }

    pub fn set_context_status(
        &self,
        tag: &ProjectTag,
        id: &ContextId,
        status: ContextStatus,
    ) -> Result<Project, WorkspaceServiceError> {
        let id = id.clone();
        self.change(
            tag,
            Box::new(move |project| project.set_status(&id, status)),
        )
    }

    pub fn set_context_static(
        &self,
        tag: &ProjectTag,
        id: &ContextId,
        is_static: bool,
    ) -> Result<Project, WorkspaceServiceError> {
        let id = id.clone();
        self.change(
            tag,
            Box::new(move |project| project.set_static(&id, is_static)),
        )
    }

    pub fn save_pull_request_draft(
        &self,
        tag: &ProjectTag,
        id: &ContextId,
        repository: &RepositoryId,
        draft: PullRequestDraft,
    ) -> Result<Project, WorkspaceServiceError> {
        let (id, repository) = (id.clone(), repository.clone());
        self.change(
            tag,
            Box::new(move |project| project.save_pull_request_draft(&id, &repository, draft)),
        )
    }

    /// `None` or a blank prompt clears it.
    pub fn set_agent_prompt(
        &self,
        tag: &ProjectTag,
        id: &ContextId,
        prompt: Option<String>,
    ) -> Result<Project, WorkspaceServiceError> {
        let id = id.clone();
        self.change(
            tag,
            Box::new(move |project| project.set_agent_prompt(&id, prompt)),
        )
    }

    pub fn record_agent_conversation(
        &self,
        tag: &ProjectTag,
        id: &ContextId,
        conversation_id: String,
    ) -> Result<Project, WorkspaceServiceError> {
        let id = id.clone();
        self.change(
            tag,
            Box::new(move |project| project.record_agent_conversation(&id, conversation_id)),
        )
    }

    /// Replaces the settings in one change, keeping the tag and the contexts.
    pub fn update_project_settings(
        &self,
        tag: &ProjectTag,
        settings: ProjectSettings,
    ) -> Result<Project, WorkspaceServiceError> {
        self.change(
            tag,
            Box::new(move |project| project.update_settings(settings)),
        )
    }

    /// Saves a new project with its default context, on each repository's
    /// default branch; repositories that can't be read are left for a later load.
    pub fn add_project(
        &self,
        tag: ProjectTag,
        settings: ProjectSettings,
    ) -> Result<Project, WorkspaceServiceError> {
        let project = Project::create(tag, settings)?;
        if self.projects.find(project.tag())?.is_some() {
            return Err(WorkspaceServiceError::ProjectAlreadyExists {
                tag: project.tag().clone(),
            });
        }
        let available_branches =
            self.available_branches(&project, &project.effective_repositories());
        let project = project.repair_default_context(&available_branches)?;
        self.projects.save(&project)?;
        self.events.project_changed(project.tag());
        Ok(project)
    }

    /// Forgets the project, leaving its folders, worktrees and branches on
    /// disk. Deleting a project that isn't there succeeds.
    pub fn delete_project(&self, tag: &ProjectTag) -> Result<(), WorkspaceServiceError> {
        self.projects.delete(tag)?;
        self.events.project_changed(tag);
        Ok(())
    }

    /// The branch name for a task title or context name under the project's
    /// naming policy; `None` asks for a name without prefix.
    pub fn suggest_branch_name(
        &self,
        tag: &ProjectTag,
        title: &str,
        prefix: Option<&str>,
    ) -> Result<BranchName, WorkspaceServiceError> {
        let project = self.find_project(tag)?;
        Ok(project.branch_naming().compose_branch_name(title, prefix)?)
    }

    /// Applies `change` to the stored project in one transaction, then tells
    /// every view.
    fn change(
        &self,
        tag: &ProjectTag,
        change: ProjectChange<'_>,
    ) -> Result<Project, WorkspaceServiceError> {
        let saved = self.projects.update(tag, change)?;
        self.events.project_changed(saved.tag());
        Ok(saved)
    }

    /// The Project as stored, without repairing its default context.
    pub fn find_project(&self, tag: &ProjectTag) -> Result<Project, WorkspaceServiceError> {
        self.projects
            .find(tag)?
            .ok_or_else(|| WorkspaceServiceError::ProjectNotFound { tag: tag.clone() })
    }

    fn open(
        &self,
        project: &Project,
        repository: &Repository,
    ) -> Result<Box<dyn GitRepository>, WorkspaceServiceError> {
        self.git
            .open(&RepositoryPath::new(project.repository_path(repository)))
            .map_err(|error| WorkspaceServiceError::git(&repository.name, error))
    }

    /// Creates the default context, or gives it a branch in the repositories
    /// it has none for, and saves it. Repositories that can't be read are
    /// left for a later load.
    fn with_repaired_default_context(
        &self,
        project: Project,
    ) -> Result<Project, WorkspaceServiceError> {
        let default_context = project.default_context();
        let unbranched: Vec<Repository> = project
            .effective_repositories()
            .iter()
            .filter(|repository| {
                default_context.is_none_or(|context| context.branch_for(&repository.id).is_none())
            })
            .cloned()
            .collect();
        if default_context.is_some() && unbranched.is_empty() {
            return Ok(project);
        }
        let available_branches = self.available_branches(&project, &unbranched);
        if project
            .clone()
            .repair_default_context(&available_branches)?
            == project
        {
            return Ok(project);
        }
        let saved = self.projects.update(
            project.tag(),
            Box::new(move |current| current.repair_default_context(&available_branches)),
        )?;
        self.events.project_changed(saved.tag());
        Ok(saved)
    }

    /// The local branches of each repository that can be opened and read.
    fn available_branches(
        &self,
        project: &Project,
        repositories: &[Repository],
    ) -> HashMap<RepositoryId, Vec<BranchName>> {
        repositories
            .iter()
            .filter_map(|repository| {
                let branches = self.open(project, repository).ok()?.branch_names().ok()?;
                Some((repository.id.clone(), branches))
            })
            .collect()
    }

    /// Opens every repository and refuses new branch names already taken,
    /// before anything is changed.
    fn plan_checkouts<'a>(
        &self,
        project: &Project,
        context: &Context,
        choices: &'a [RepositoryBranchChoice],
    ) -> Result<Vec<PlannedCheckout<'a>>, WorkspaceServiceError> {
        let mut planned = Vec::with_capacity(choices.len());
        for choice in choices {
            let repository = project
                .find_effective_repository(&choice.repository)
                .expect("the Project accepted the context's repositories")
                .into_owned();
            let git = self.open(project, &repository)?;
            if let BranchChoice::NewBranch { name, .. } = &choice.choice {
                if git.has_branch(name) {
                    return Err(WorkspaceServiceError::NewBranchAlreadyExists {
                        repository: repository.name.clone(),
                        branch: name.clone(),
                    });
                }
            }
            let worktree = WorktreePath::new(project.worktree_path(context, &repository));
            planned.push(PlannedCheckout {
                repository,
                git,
                choice: &choice.choice,
                worktree,
            });
        }
        Ok(planned)
    }

    /// Fetches the head and base branch of every matched pull request from
    /// `origin`, before anything is checked out. A fetch is a no-op when the
    /// branch already exists locally. Any failure stops the create, so nothing
    /// is made; the error names the pull request and the branch.
    fn fetch_pull_request_branches(
        &self,
        planned: &[PlannedCheckout<'_>],
    ) -> Result<(), WorkspaceServiceError> {
        for checkout in planned {
            let BranchChoice::PullRequest {
                head,
                base,
                matched,
            } = checkout.choice
            else {
                continue;
            };
            for branch in [head, base] {
                checkout.git.fetch_branch(branch).map_err(|error| {
                    WorkspaceServiceError::PullRequestBranchNotFetched {
                        repository: checkout.repository.name.clone(),
                        pull_request_number: matched.number,
                        branch: branch.clone(),
                        error,
                    }
                })?;
            }
        }
        Ok(())
    }

    fn check_out<'a>(
        &self,
        project: &Project,
        context: &Context,
        planned: &'a [PlannedCheckout<'a>],
        journal: &mut CheckoutJournal<'a>,
    ) -> Result<(), WorkspaceServiceError> {
        for checkout in planned {
            let repository_name = &checkout.repository.name;
            if let BranchChoice::NewBranch { name, base_branch } = checkout.choice {
                checkout
                    .git
                    .create_branch(name, base_branch)
                    .map_err(|error| match error {
                        GitError::BranchAlreadyExists { .. } => {
                            WorkspaceServiceError::NewBranchAlreadyExists {
                                repository: repository_name.clone(),
                                branch: name.clone(),
                            }
                        }
                        other => WorkspaceServiceError::git(repository_name, other),
                    })?;
                journal.created_branch(checkout.git.as_ref(), name);
            }
            let branch = checkout.choice.branch();
            checkout
                .git
                .add_worktree(&checkout.worktree, branch)
                .map_err(|error| match error {
                    GitError::BranchCheckedOut { worktree, .. } => {
                        WorkspaceServiceError::BranchCheckedOut {
                            repository: repository_name.clone(),
                            branch: branch.clone(),
                            checked_out_in: checkout_holder(
                                project,
                                &checkout.repository,
                                worktree,
                            ),
                        }
                    }
                    other => WorkspaceServiceError::git(repository_name, other),
                })?;
            journal.added_worktree(checkout.git.as_ref(), &checkout.worktree);
        }

        for checkout in planned {
            if let Some(command) = &checkout.repository.post_checkout_command {
                self.files
                    .run_post_checkout(command, checkout.worktree.as_path());
            }
        }
        for configured in project.configured_links(context) {
            let link = Path::new(&configured.link);
            if self
                .files
                .create_symlink(Path::new(&configured.target), link)?
            {
                journal.created_symlink(link.to_path_buf());
            }
        }
        Ok(())
    }

    /// Removes the repository's checkout in the context: the worktree, or the
    /// symlink of a legacy linked repository. A worktree git no longer has is
    /// pruned. Returns the opened repository, none for a linked one.
    fn remove_checkout(
        &self,
        project: &Project,
        context: &Context,
        repository: &Repository,
        branch: &ContextBranch,
    ) -> Result<Option<Box<dyn GitRepository>>, WorkspaceServiceError> {
        let worktree = project.worktree_path(context, repository);
        if branch.is_linked() {
            self.files.remove_symlink(Path::new(&worktree))?;
            return Ok(None);
        }
        let git = self.open(project, repository)?;
        if let Err(error) = git.remove_worktree(&WorktreePath::new(&worktree)) {
            let still_checked_out = !matches!(git.worktree_for(&branch.branch), Ok(None));
            if still_checked_out {
                return Err(WorkspaceServiceError::git(&repository.name, error));
            }
            let _ = git.prune_worktrees();
        }
        Ok(Some(git))
    }

    fn remove_context_folder(&self, project: &Project, context: &Context) {
        for configured in project.configured_links(context) {
            if let Err(error) = self.files.remove_symlink(Path::new(&configured.link)) {
                eprintln!("[workspace] {error}");
            }
        }
        self.files
            .remove_folder_if_empty(Path::new(&project.context_folder_path(context)));
    }

    /// Deletes the context's branch when it never got a commit beyond its
    /// base, was never pushed, isn't checked out anywhere and no other context
    /// uses it. Any doubt keeps the branch.
    fn delete_branch_if_unused(
        &self,
        project: &Project,
        context: &Context,
        checkout: &RemovedCheckout<'_>,
    ) {
        let Some(git) = &checkout.git else {
            return;
        };
        let branch = &checkout.branch.branch;
        let Some(base) = base_branch(project, context, &checkout.repository.id) else {
            return;
        };
        let used_elsewhere = project.contexts().iter().any(|other| {
            other.id != context.id
                && other
                    .branch_for(&checkout.repository.id)
                    .is_some_and(|other_branch| &other_branch.branch == branch)
        });
        if base == branch || used_elsewhere {
            return;
        }
        let is_unpublished_and_free = git.branches().is_ok_and(|branches| {
            branches.iter().any(|local| {
                &local.name == branch
                    && !local.has_upstream
                    && !local.upstream_gone
                    && !local.is_checked_out
            })
        });
        let has_no_work = matches!(git.has_commits_beyond(branch, base), Ok(false));
        if is_unpublished_and_free && has_no_work {
            if let Err(error) = git.delete_branch(branch, true) {
                eprintln!(
                    "[workspace] branch '{branch}' of '{}' kept: {error}",
                    checkout.repository.name
                );
            }
        }
    }
}

/// The base context's branch in the repository, else the default context's.
fn base_branch<'a>(
    project: &'a Project,
    context: &Context,
    repository: &RepositoryId,
) -> Option<&'a BranchName> {
    project.base_branch_for(context, repository)
}

/// The context of the project whose checkout of `repository` is at
/// `worktree`, else the folder itself.
fn checkout_holder(project: &Project, repository: &Repository, worktree: String) -> CheckoutHolder {
    let checked_out_at = WorktreePath::new(&worktree).display();
    project
        .contexts()
        .iter()
        .find(|context| {
            context.branch_for(&repository.id).is_some()
                && WorktreePath::new(project.worktree_path(context, repository)).display()
                    == checked_out_at
        })
        .map(|context| CheckoutHolder::Context(context.name.clone()))
        .unwrap_or(CheckoutHolder::Folder(worktree))
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
