//! TEMPORARY: one-time import of the old `projects/<tag>.json` files into the
//! project repository; delete once every user has run the import; see #36.
//!
//! Each file is a tauri-plugin-store file: a JSON object whose `"project"`
//! entry holds the frontend's former `ProjectData`. Every readable file is
//! converted to a `Project`, saved, then renamed `<tag>.json.imported`.
//! Unreadable files are left untouched and reported; one bad file never stops
//! the others. A context the domain refuses is skipped and reported, and the
//! rest of its project is still imported.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::work_tracking::domain::provider_kind::ProviderKind;
use crate::workspace::domain::agent_work::AgentWork;
use crate::workspace::domain::branch_naming::{parse_branch_name, BranchNamingPolicy};
use crate::workspace::domain::context::Context;
use crate::workspace::domain::context_branch::{
    ContextBranch, ContextCheckout, PullRequestDraft, PullRequestLink,
};
use crate::workspace::domain::context_folder::ContextFolder;
use crate::workspace::domain::context_name::ContextName;
use crate::workspace::domain::context_origin::{ContextOrigin, TaskRef};
use crate::workspace::domain::context_status::ContextStatus;
use crate::workspace::domain::error::WorkspaceError;
use crate::workspace::domain::identifiers::{ContextId, RepositoryId};
use crate::workspace::domain::ports::{ProjectRepository, StorageError};
use crate::workspace::domain::project::{Project, ProjectSettings};
use crate::workspace::domain::project_tag::ProjectTag;
use crate::workspace::domain::remote_project_link::RemoteProjectLink;
use crate::workspace::domain::repository::Repository;
use crate::workspace::domain::slug::slugify;

pub const IMPORTED_SUFFIX: &str = ".imported";
const JSON_EXTENSION: &str = "json";
const PROJECT_ENTRY: &str = "project";
const UNNAMED_CONTEXT: &str = "(unnamed)";
const MAX_REMOTE_ID_LENGTH: usize = 300;
const MAX_REMOTE_NAME_LENGTH: usize = 200;
const HTTPS_SCHEME: &str = "https";

/// What one import run did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportReport {
    /// The Projects saved, in file name order.
    pub imported: Vec<ProjectTag>,
    /// Files left where they were, with why: unreadable, refused or not saved,
    /// or saved but not renamed (that Project is also in `imported`).
    pub unreadable: Vec<UnreadableFile>,
    /// Contexts of imported Projects that the domain refused.
    pub skipped_contexts: Vec<SkippedContext>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableFile {
    pub file_name: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedContext {
    pub project: ProjectTag,
    pub context_name: String,
    pub reason: String,
}

/// Imports the project files of `projects_folder` on the first launch with
/// the database: only when the folder exists and the repository holds no
/// record at all, readable or not. Otherwise does nothing and reports nothing.
pub fn import_if_needed(
    projects_folder: &Path,
    repository: &dyn ProjectRepository,
) -> Result<ImportReport, StorageError> {
    if !projects_folder.is_dir() {
        return Ok(ImportReport::default());
    }
    let listing = repository.list()?;
    if !listing.projects.is_empty() || !listing.unreadable.is_empty() {
        return Ok(ImportReport::default());
    }
    import_project_files(projects_folder, repository)
}

fn import_project_files(
    projects_folder: &Path,
    repository: &dyn ProjectRepository,
) -> Result<ImportReport, StorageError> {
    let mut report = ImportReport::default();
    for file in project_files(projects_folder)? {
        let file_name = file_name_of(&file);
        match import_file(&file, repository, &report.imported) {
            Ok(imported) => {
                if let Err(error) = fs::rename(&file, imported_path(&file)) {
                    report.unreadable.push(UnreadableFile {
                        file_name,
                        reason: format!("Imported, but couldn't be renamed: {error}"),
                    });
                }
                report.imported.push(imported.tag);
                report.skipped_contexts.extend(imported.skipped_contexts);
            }
            Err(reason) => report.unreadable.push(UnreadableFile { file_name, reason }),
        }
    }
    Ok(report)
}

fn project_files(projects_folder: &Path) -> Result<Vec<PathBuf>, StorageError> {
    let unreadable_folder = |error: std::io::Error| StorageError::Unavailable {
        reason: format!("The projects folder couldn't be read: {error}"),
    };
    let mut files = Vec::new();
    for entry in fs::read_dir(projects_folder).map_err(unreadable_folder)? {
        let path = entry.map_err(unreadable_folder)?.path();
        let is_json = path
            .extension()
            .is_some_and(|extension| extension == JSON_EXTENSION);
        if is_json && path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn imported_path(file: &Path) -> PathBuf {
    let mut imported = file.as_os_str().to_owned();
    imported.push(IMPORTED_SUFFIX);
    PathBuf::from(imported)
}

struct ImportedProject {
    tag: ProjectTag,
    skipped_contexts: Vec<SkippedContext>,
}

fn import_file(
    file: &Path,
    repository: &dyn ProjectRepository,
    already_imported: &[ProjectTag],
) -> Result<ImportedProject, String> {
    let contents =
        fs::read_to_string(file).map_err(|error| format!("The file couldn't be read: {error}"))?;
    let mut store: HashMap<String, Value> = serde_json::from_str(&contents)
        .map_err(|error| format!("The file is not a valid project store: {error}"))?;
    let project_data = store
        .remove(PROJECT_ENTRY)
        .filter(|value| !value.is_null())
        .ok_or_else(|| format!("The file has no \"{PROJECT_ENTRY}\" entry"))?;
    let legacy_project: LegacyProject = serde_json::from_value(project_data)
        .map_err(|error| format!("The project is incomplete: {error}"))?;
    let file_stem = file
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (project, skipped_contexts) =
        convert_project(legacy_project, &file_stem).map_err(|error| error.to_string())?;
    if already_imported.contains(project.tag()) {
        return Err(format!(
            "Another file already imported a project tagged '{}'",
            project.tag()
        ));
    }
    repository
        .save(&project)
        .map_err(|error| format!("The project couldn't be saved: {error}"))?;
    Ok(ImportedProject {
        tag: project.tag().clone(),
        skipped_contexts,
    })
}

/// The frontend's former `ProjectData`. Contexts and remote project links are
/// read one by one, so a bad entry only loses itself.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyProject {
    name: String,
    tag: Option<String>,
    path: String,
    icon: Option<String>,
    repositories: Option<Vec<LegacyRepository>>,
    contexts: Option<Vec<Value>>,
    symlinks: Option<Vec<String>>,
    remote_project_links: Option<Vec<Value>>,
    branch_naming: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyRepository {
    id: String,
    name: String,
    rel_path: String,
    post_checkout_command: Option<String>,
}

/// The frontend's former `ContextData`; `expanded` is dropped by not being read.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyContext {
    id: String,
    name: String,
    branches: Option<Vec<LegacyContextBranch>>,
    is_default: Option<bool>,
    base_context_name: Option<String>,
    pull_request_drafts: Option<HashMap<String, LegacyPullRequestDraft>>,
    status: Option<Value>,
    preprompt: Option<String>,
    work_item_ref: Option<Value>,
    is_static: Option<bool>,
    kiro_conversation_id: Option<String>,
    folder_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyContextBranch {
    repository_id: String,
    branch: String,
    linked: Option<bool>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct LegacyPullRequestDraft {
    title: String,
    description: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyWorkItemRef {
    connection_id: String,
    work_item_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyRemoteProjectLink {
    provider_kind: String,
    connection_id: String,
    remote_project_id: String,
    remote_project_name: String,
    remote_project_url: Option<Value>,
    linked_at: String,
}

fn convert_project(
    legacy: LegacyProject,
    file_stem: &str,
) -> Result<(Project, Vec<SkippedContext>), WorkspaceError> {
    let tag = legacy
        .tag
        .as_deref()
        .filter(|tag| !tag.trim().is_empty())
        .unwrap_or(file_stem);
    let tag = ProjectTag::parse(tag)?;
    let repositories = legacy
        .repositories
        .unwrap_or_default()
        .into_iter()
        .map(convert_repository)
        .collect::<Result<Vec<_>, _>>()?;
    let branch_naming = legacy
        .branch_naming
        .and_then(|value| serde_json::from_value::<BranchNamingPolicy>(value).ok())
        .unwrap_or_default();
    let settings = ProjectSettings {
        name: legacy.name,
        path: legacy.path,
        icon: legacy.icon,
        repositories,
        symlinks: legacy.symlinks.unwrap_or_default(),
        remote_project_links: convert_remote_project_links(
            legacy.remote_project_links.unwrap_or_default(),
        ),
        branch_naming,
    };
    let project = Project::new(tag.clone(), settings);
    let mut importer = ContextImporter {
        project,
        skipped: Vec::new(),
    };
    importer.import(legacy.contexts.unwrap_or_default());
    Ok((importer.project, importer.skipped))
}

fn convert_repository(legacy: LegacyRepository) -> Result<Repository, WorkspaceError> {
    Ok(Repository {
        post_checkout_command: legacy.post_checkout_command,
        ..Repository::new(
            RepositoryId::parse(&legacy.id)?,
            legacy.name,
            legacy.rel_path,
        )
    })
}

/// Valid links only, the first of each identity kept, as the frontend's
/// former `RemoteProjectLinks.fromJSON` did.
fn convert_remote_project_links(entries: Vec<Value>) -> Vec<RemoteProjectLink> {
    let mut links: Vec<RemoteProjectLink> = Vec::new();
    for entry in entries {
        let Some(link) = serde_json::from_value::<LegacyRemoteProjectLink>(entry)
            .ok()
            .and_then(convert_remote_project_link)
        else {
            continue;
        };
        let is_duplicate = links.iter().any(|existing| {
            existing.provider_kind == link.provider_kind
                && existing.connection_id == link.connection_id
                && existing.remote_project_id == link.remote_project_id
        });
        if !is_duplicate {
            links.push(link);
        }
    }
    links
}

fn convert_remote_project_link(legacy: LegacyRemoteProjectLink) -> Option<RemoteProjectLink> {
    let provider_kind = ProviderKind::parse(&legacy.provider_kind).ok()?;
    Some(RemoteProjectLink {
        provider_kind: provider_kind.as_str().to_string(),
        connection_id: bounded_text(&legacy.connection_id, MAX_REMOTE_ID_LENGTH)?,
        remote_project_id: bounded_text(&legacy.remote_project_id, MAX_REMOTE_ID_LENGTH)?,
        remote_project_name: bounded_text(&legacy.remote_project_name, MAX_REMOTE_NAME_LENGTH)?,
        remote_project_url: legacy.remote_project_url.as_ref().and_then(https_url),
        linked_at: is_timestamp(&legacy.linked_at).then_some(legacy.linked_at)?,
    })
}

fn bounded_text(raw: &str, max_length: usize) -> Option<String> {
    let trimmed = raw.trim();
    let is_valid = !trimmed.is_empty() && trimmed.chars().count() <= max_length;
    is_valid.then(|| trimmed.to_string())
}

fn https_url(value: &Value) -> Option<String> {
    let trimmed = value.as_str()?.trim();
    let url = reqwest::Url::parse(trimmed).ok()?;
    (url.scheme() == HTTPS_SCHEME).then(|| trimmed.to_string())
}

/// The frontend wrote `Date.toISOString()`; an ISO calendar date at the
/// start is enough to accept it.
fn is_timestamp(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    let digits_at = |range: std::ops::Range<usize>| {
        bytes
            .get(range)
            .is_some_and(|digits| digits.iter().all(u8::is_ascii_digit))
    };
    digits_at(0..4)
        && bytes.get(4) == Some(&b'-')
        && digits_at(5..7)
        && bytes.get(7) == Some(&b'-')
        && digits_at(8..10)
}

/// A readable old context, waiting for its base context to be added first.
struct PendingContext {
    legacy_id: String,
    name: String,
    folder: String,
    base_legacy_id: Option<String>,
    legacy: LegacyContext,
}

struct ContextImporter {
    project: Project,
    skipped: Vec<SkippedContext>,
}

impl ContextImporter {
    fn import(&mut self, entries: Vec<Value>) {
        let mut pending = self.read_contexts(entries);
        resolve_base_contexts(&mut pending);
        let mut added: HashMap<String, ContextId> = HashMap::new();
        while !pending.is_empty() {
            let next = next_ready(&pending);
            let context = pending.remove(next);
            let base = context
                .base_legacy_id
                .as_ref()
                .and_then(|legacy_id| added.get(legacy_id))
                .cloned();
            let legacy_id = context.legacy_id.clone();
            if let Some(id) = self.add(context, base) {
                added.insert(legacy_id, id);
            }
        }
    }

    fn read_contexts(&mut self, entries: Vec<Value>) -> Vec<PendingContext> {
        let mut pending: Vec<PendingContext> = Vec::new();
        for entry in entries {
            let context_name = entry
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(UNNAMED_CONTEXT)
                .to_string();
            let legacy = match serde_json::from_value::<LegacyContext>(entry) {
                Ok(legacy) => legacy,
                Err(error) => {
                    self.skip(context_name, format!("The context is incomplete: {error}"));
                    continue;
                }
            };
            if pending.iter().any(|other| other.legacy_id == legacy.id) {
                self.skip(
                    context_name,
                    format!("Another context has the id '{}'", legacy.id),
                );
                continue;
            }
            pending.push(PendingContext {
                legacy_id: legacy.id.clone(),
                folder: stored_folder_name(&legacy)
                    .unwrap_or(&legacy.name)
                    .to_string(),
                name: legacy.name.clone(),
                base_legacy_id: None,
                legacy,
            });
        }
        pending
    }

    /// Adds the context, or records why the domain refused it.
    fn add(&mut self, pending: PendingContext, base: Option<ContextId>) -> Option<ContextId> {
        let result = convert_context(pending.legacy, base).and_then(|context| {
            let id = context.id.clone();
            let project = self.project.clone().add_legacy_context(context)?;
            Ok((project, id))
        });
        match result {
            Ok((project, id)) => {
                self.project = project;
                Some(id)
            }
            Err(error) => {
                self.skip(pending.name, error.to_string());
                None
            }
        }
    }

    fn skip(&mut self, context_name: String, reason: String) {
        self.skipped.push(SkippedContext {
            project: self.project.tag().clone(),
            context_name,
            reason,
        });
    }
}

fn stored_folder_name(legacy: &LegacyContext) -> Option<&str> {
    legacy
        .folder_name
        .as_deref()
        .filter(|folder| !folder.trim().is_empty())
}

/// Matches each base context name to another context: by exact name, then by
/// name ignoring case, then by folder. Unknown names are dropped.
fn resolve_base_contexts(pending: &mut [PendingContext]) {
    let resolved: Vec<Option<String>> = pending
        .iter()
        .map(|context| {
            let base_name = context.legacy.base_context_name.as_deref()?.trim();
            let others = || {
                pending
                    .iter()
                    .filter(|other| other.legacy_id != context.legacy_id)
            };
            let base_slug = slugify(base_name);
            others()
                .find(|other| other.name == base_name)
                .or_else(|| {
                    others().find(|other| other.name.to_lowercase() == base_name.to_lowercase())
                })
                .or_else(|| {
                    others().find(|other| {
                        let folder = other.folder.to_lowercase();
                        folder == base_name.to_lowercase()
                            || (!base_slug.is_empty() && folder == base_slug)
                    })
                })
                .map(|base| base.legacy_id.clone())
        })
        .collect();
    for (context, base_legacy_id) in pending.iter_mut().zip(resolved) {
        context.base_legacy_id = base_legacy_id;
    }
}

/// The first context whose base context is no longer waiting, so bases are
/// added before the contexts based on them. Bases that wait on each other
/// are broken by taking the first one.
fn next_ready(pending: &[PendingContext]) -> usize {
    let waiting: HashSet<&str> = pending
        .iter()
        .map(|context| context.legacy_id.as_str())
        .collect();
    pending
        .iter()
        .position(|context| {
            context
                .base_legacy_id
                .as_deref()
                .is_none_or(|base| !waiting.contains(base))
        })
        .unwrap_or(0)
}

fn convert_context(
    legacy: LegacyContext,
    base: Option<ContextId>,
) -> Result<Context, WorkspaceError> {
    let name = ContextName::parse(&legacy.name)?;
    let folder = match stored_folder_name(&legacy) {
        Some(folder_name) => ContextFolder::from_stored(folder_name)?,
        None => ContextFolder::legacy(&name),
    };
    let is_default = legacy.is_default.unwrap_or(false);
    let status = legacy
        .status
        .and_then(|status| serde_json::from_value::<ContextStatus>(status).ok())
        .unwrap_or(if is_default {
            ContextStatus::Idle
        } else {
            ContextStatus::default()
        });
    let mut drafts = legacy.pull_request_drafts.unwrap_or_default();
    let branches = legacy
        .branches
        .unwrap_or_default()
        .into_iter()
        .map(|branch| convert_branch(branch, &mut drafts))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Context {
        id: ContextId::parse(&legacy.id)?,
        name,
        folder,
        status,
        is_default,
        is_static: legacy.is_static.unwrap_or(false),
        base,
        branches,
        origin: convert_origin(legacy.work_item_ref),
        agent: convert_agent_work(legacy.preprompt, legacy.kiro_conversation_id),
    })
}

/// Linked repositories keep no draft: their pull request belongs to the
/// context they link to.
fn convert_branch(
    legacy: LegacyContextBranch,
    drafts: &mut HashMap<String, LegacyPullRequestDraft>,
) -> Result<ContextBranch, WorkspaceError> {
    let is_linked = legacy.linked.unwrap_or(false);
    let draft = drafts.remove(&legacy.repository_id).filter(|_| !is_linked);
    Ok(ContextBranch {
        pull_request: draft.map(|draft| PullRequestLink {
            draft: PullRequestDraft {
                title: draft.title,
                description: draft.description,
            },
        }),
        checkout: if is_linked {
            ContextCheckout::Linked
        } else {
            ContextCheckout::Worktree
        },
        ..ContextBranch::new(
            RepositoryId::parse(&legacy.repository_id)?,
            parse_branch_name(&legacy.branch)?,
        )
    })
}

fn convert_origin(work_item_ref: Option<Value>) -> ContextOrigin {
    work_item_ref
        .and_then(|value| serde_json::from_value::<LegacyWorkItemRef>(value).ok())
        .map(|reference| {
            ContextOrigin::FromTask(TaskRef {
                connection_id: reference.connection_id,
                task_id: reference.work_item_id,
            })
        })
        .unwrap_or_default()
}

fn convert_agent_work(
    preprompt: Option<String>,
    conversation_id: Option<String>,
) -> Option<AgentWork> {
    let agent = AgentWork::default().with_prompt(preprompt);
    let agent = match conversation_id.filter(|id| !id.trim().is_empty()) {
        Some(conversation_id) => agent.with_conversation(conversation_id),
        None => agent,
    };
    (agent != AgentWork::default()).then_some(agent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::domain::agent_work::AgentTool;
    use crate::workspace::infra::redb_project_repository::RedbProjectRepository;
    use tempfile::TempDir;

    const FIXTURES_FOLDER: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/workspace/infra/fixtures/legacy"
    );
    const DATABASE_FILE: &str = "me.redb";
    const PROJECTS_FOLDER: &str = "projects";

    const ACME: &str = "acme";
    const SOLO: &str = "solo";
    const DUPLICATES: &str = "duplicates";
    const BROKEN_FILE: &str = "broken.json";
    const NO_PROJECT_FILE: &str = "no-project.json";

    const API: &str = "4b1c8a52-1f0e-4d43-9f43-0d1c2a7a1001";
    const WEB: &str = "4b1c8a52-1f0e-4d43-9f43-0d1c2a7a1002";
    const LOGIN_PAGE_ID: &str = "8f6a2e10-0000-4000-8000-000000000002";
    const DEFAULT_CONTEXT: &str = "default";
    const LOGIN_PAGE: &str = "Login page";
    const LOGIN_FOLLOW_UP: &str = "Login follow-up";
    const RELEASE: &str = "Release";
    const DOCS: &str = "Docs";
    const SPIKE: &str = "Spike";
    const FEATURE_A: &str = "Feature A";

    struct Imported {
        report: ImportReport,
        repository: RedbProjectRepository,
        projects_folder: PathBuf,
        _folder: TempDir,
    }

    impl Imported {
        fn project(&self, tag: &str) -> Project {
            self.repository
                .find(&ProjectTag::parse(tag).unwrap())
                .unwrap()
                .unwrap()
        }

        fn context(&self, tag: &str, name: &str) -> Context {
            self.project(tag)
                .contexts()
                .iter()
                .find(|context| context.name.as_str() == name)
                .cloned()
                .unwrap()
        }

        fn file_exists(&self, file_name: &str) -> bool {
            self.projects_folder.join(file_name).exists()
        }

        fn import_again(&self) -> ImportReport {
            import_if_needed(&self.projects_folder, &self.repository).unwrap()
        }
    }

    fn copy_fixtures(projects_folder: &Path) {
        fs::create_dir_all(projects_folder).unwrap();
        for entry in fs::read_dir(FIXTURES_FOLDER).unwrap() {
            let fixture = entry.unwrap().path();
            fs::copy(&fixture, projects_folder.join(fixture.file_name().unwrap())).unwrap();
        }
    }

    fn import_fixtures() -> Imported {
        let folder = TempDir::new().unwrap();
        let projects_folder = folder.path().join(PROJECTS_FOLDER);
        copy_fixtures(&projects_folder);
        let repository = RedbProjectRepository::open(folder.path().join(DATABASE_FILE)).unwrap();
        let report = import_if_needed(&projects_folder, &repository).unwrap();
        Imported {
            report,
            repository,
            projects_folder,
            _folder: folder,
        }
    }

    fn tags(tags: &[&str]) -> Vec<ProjectTag> {
        tags.iter()
            .map(|tag| ProjectTag::parse(tag).unwrap())
            .collect()
    }

    fn repository_id(raw: &str) -> RepositoryId {
        RepositoryId::parse(raw).unwrap()
    }

    fn draft_of(context: &Context, repository: &str) -> Option<PullRequestDraft> {
        context
            .branch_for(&repository_id(repository))
            .unwrap()
            .pull_request
            .clone()
            .map(|link| link.draft)
    }

    fn unreadable_file_names(report: &ImportReport) -> Vec<&str> {
        report
            .unreadable
            .iter()
            .map(|file| file.file_name.as_str())
            .collect()
    }

    mod files {
        use super::*;

        #[test]
        fn imports_every_readable_file() {
            let imported = import_fixtures();
            assert_eq!(imported.report.imported, tags(&[ACME, DUPLICATES, SOLO]));
            assert_eq!(imported.repository.list().unwrap().projects.len(), 3);
        }

        #[test]
        fn renames_imported_files() {
            let imported = import_fixtures();
            assert!(!imported.file_exists("acme.json"));
            assert!(imported.file_exists("acme.json.imported"));
            assert!(imported.file_exists("solo.json.imported"));
        }

        #[test]
        fn reports_unreadable_files_with_their_reason() {
            let imported = import_fixtures();
            assert_eq!(
                unreadable_file_names(&imported.report),
                [BROKEN_FILE, NO_PROJECT_FILE]
            );
            assert!(imported.report.unreadable[0]
                .reason
                .contains("not a valid project store"));
            assert_eq!(
                imported.report.unreadable[1].reason,
                "The file has no \"project\" entry"
            );
        }

        #[test]
        fn leaves_unreadable_files_untouched() {
            let imported = import_fixtures();
            let original = fs::read(Path::new(FIXTURES_FOLDER).join(BROKEN_FILE)).unwrap();
            assert_eq!(
                fs::read(imported.projects_folder.join(BROKEN_FILE)).unwrap(),
                original
            );
            assert!(imported.file_exists(NO_PROJECT_FILE));
        }

        #[test]
        fn a_second_run_does_nothing() {
            let imported = import_fixtures();
            let projects_before = imported.repository.list().unwrap();
            assert_eq!(imported.import_again(), ImportReport::default());
            assert_eq!(imported.repository.list().unwrap(), projects_before);
            assert!(imported.file_exists(BROKEN_FILE));
        }

        #[test]
        fn does_nothing_when_the_database_already_has_projects() {
            let folder = TempDir::new().unwrap();
            let projects_folder = folder.path().join(PROJECTS_FOLDER);
            copy_fixtures(&projects_folder);
            let repository =
                RedbProjectRepository::open(folder.path().join(DATABASE_FILE)).unwrap();
            repository
                .save(&Project::new(
                    ProjectTag::parse(SOLO).unwrap(),
                    crate::workspace::domain::test_support::settings(vec![]),
                ))
                .unwrap();
            let report = import_if_needed(&projects_folder, &repository).unwrap();
            assert_eq!(report, ImportReport::default());
            assert!(projects_folder.join("acme.json").exists());
        }

        #[test]
        fn does_nothing_without_a_projects_folder() {
            let folder = TempDir::new().unwrap();
            let repository =
                RedbProjectRepository::open(folder.path().join(DATABASE_FILE)).unwrap();
            let report =
                import_if_needed(&folder.path().join(PROJECTS_FOLDER), &repository).unwrap();
            assert_eq!(report, ImportReport::default());
        }
    }

    mod project_settings {
        use super::*;

        #[test]
        fn keeps_the_project_details() {
            let project = import_fixtures().project(ACME);
            assert_eq!(project.name(), "Acme");
            assert_eq!(project.path(), "/Users/me/projects/acme");
            assert_eq!(project.icon(), Some("data:image/png;base64,iVBORw0KGgo="));
            assert_eq!(project.symlinks(), [".env", "node_modules"]);
        }

        #[test]
        fn keeps_the_repositories_with_their_post_checkout_command() {
            let project = import_fixtures().project(ACME);
            let api = &project.repositories()[0];
            assert_eq!(project.repositories().len(), 2);
            assert_eq!(api.id, repository_id(API));
            assert_eq!(api.relative_path, "./api");
            assert_eq!(api.post_checkout_command.as_deref(), Some("pnpm install"));
            assert_eq!(project.repositories()[1].post_checkout_command, None);
        }

        #[test]
        fn keeps_the_branch_naming_policy() {
            let project = import_fixtures().project(ACME);
            assert_eq!(
                project.branch_naming(),
                &BranchNamingPolicy::new(&["feat", "bugfix"], false)
            );
        }

        #[test]
        fn defaults_a_missing_branch_naming_policy() {
            let project = import_fixtures().project(SOLO);
            assert_eq!(project.branch_naming(), &BranchNamingPolicy::default());
        }

        #[test]
        fn keeps_valid_remote_project_links_once_each() {
            let project = import_fixtures().project(ACME);
            let links = project.remote_project_links();
            assert_eq!(links.len(), 1);
            assert_eq!(links[0].remote_project_name, "Acme website");
            assert_eq!(
                links[0].remote_project_url.as_deref(),
                Some("https://acme.teamwork.com/app/projects/42")
            );
        }

        #[test]
        fn a_project_without_repositories_uses_its_root_repository() {
            let imported = import_fixtures();
            let spike = imported.context(SOLO, SPIKE);
            assert!(imported.project(SOLO).repositories().is_empty());
            assert_eq!(spike.branches[0].repository, repository_id(SOLO));
            assert_eq!(spike.branches[0].branch.as_str(), "spike");
        }
    }

    mod contexts {
        use super::*;

        #[test]
        fn imports_every_context() {
            let project = import_fixtures().project(ACME);
            assert_eq!(project.contexts().len(), 5);
        }

        #[test]
        fn keeps_the_context_ids() {
            let login_page = import_fixtures().context(ACME, LOGIN_PAGE);
            assert_eq!(login_page.id.as_str(), LOGIN_PAGE_ID);
        }

        #[test]
        fn keeps_the_default_context_idle_and_static() {
            let project = import_fixtures().project(ACME);
            let default_context = project.default_context().unwrap();
            assert_eq!(default_context.name.as_str(), DEFAULT_CONTEXT);
            assert_eq!(default_context.status, ContextStatus::Idle);
            assert!(default_context.is_static);
        }

        #[test]
        fn a_default_context_without_status_is_idle() {
            let project = import_fixtures().project(SOLO);
            assert_eq!(
                project.default_context().unwrap().status,
                ContextStatus::Idle
            );
        }

        #[test]
        fn keeps_linked_repositories_as_linked() {
            let follow_up = import_fixtures().context(ACME, LOGIN_FOLLOW_UP);
            let web = follow_up.branch_for(&repository_id(WEB)).unwrap();
            let api = follow_up.branch_for(&repository_id(API)).unwrap();
            assert_eq!(web.checkout, ContextCheckout::Linked);
            assert_eq!(api.checkout, ContextCheckout::Worktree);
        }

        #[test]
        fn finds_the_base_context_by_name_even_when_listed_later() {
            let follow_up = import_fixtures().context(ACME, LOGIN_FOLLOW_UP);
            assert_eq!(follow_up.base.unwrap().as_str(), LOGIN_PAGE_ID);
        }

        #[test]
        fn finds_the_base_context_by_folder() {
            let docs = import_fixtures().context(ACME, DOCS);
            assert_eq!(docs.base.unwrap().as_str(), LOGIN_PAGE_ID);
        }

        #[test]
        fn drops_an_unknown_base_context() {
            let release = import_fixtures().context(ACME, RELEASE);
            assert_eq!(release.base, None);
        }

        #[test]
        fn moves_pull_request_drafts_onto_their_branch() {
            let login_page = import_fixtures().context(ACME, LOGIN_PAGE);
            assert_eq!(
                draft_of(&login_page, API),
                Some(PullRequestDraft {
                    title: "Add the login endpoint".to_string(),
                    description: "Adds POST /login.".to_string(),
                })
            );
            assert_eq!(draft_of(&login_page, WEB), None);
        }

        #[test]
        fn drops_drafts_of_linked_repositories() {
            let follow_up = import_fixtures().context(ACME, LOGIN_FOLLOW_UP);
            assert_eq!(draft_of(&follow_up, WEB), None);
        }

        #[test]
        fn keeps_a_stored_folder() {
            let login_page = import_fixtures().context(ACME, LOGIN_PAGE);
            assert_eq!(login_page.folder.as_str(), "login-page");
        }

        #[test]
        fn uses_the_raw_name_as_folder_when_none_is_stored() {
            let follow_up = import_fixtures().context(ACME, LOGIN_FOLLOW_UP);
            assert_eq!(follow_up.folder.as_str(), LOGIN_FOLLOW_UP);
        }

        #[test]
        fn turns_the_prompt_and_conversation_into_agent_work() {
            let login_page = import_fixtures().context(ACME, LOGIN_PAGE);
            assert_eq!(
                login_page.agent,
                Some(AgentWork {
                    prompt: Some("Build the login page".to_string()),
                    tool: AgentTool::Kiro {
                        conversation_id: Some("kiro-conversation-1".to_string())
                    },
                })
            );
        }

        #[test]
        fn turns_a_conversation_alone_into_agent_work() {
            let docs = import_fixtures().context(ACME, DOCS);
            assert_eq!(
                docs.agent,
                Some(AgentWork::default().with_conversation("kiro-conversation-2".to_string()))
            );
        }

        #[test]
        fn has_no_agent_work_without_prompt_or_conversation() {
            let release = import_fixtures().context(ACME, RELEASE);
            assert_eq!(release.agent, None);
        }

        #[test]
        fn remembers_the_task_a_context_was_created_from() {
            let login_page = import_fixtures().context(ACME, LOGIN_PAGE);
            assert_eq!(
                login_page.origin,
                ContextOrigin::FromTask(TaskRef {
                    connection_id: "teamwork:acme.teamwork.com".to_string(),
                    task_id: "101".to_string(),
                })
            );
        }

        #[test]
        fn a_malformed_task_reference_gives_a_manual_context() {
            let release = import_fixtures().context(ACME, RELEASE);
            assert_eq!(release.origin, ContextOrigin::Manual);
        }

        #[test]
        fn keeps_a_valid_status() {
            let login_page = import_fixtures().context(ACME, LOGIN_PAGE);
            assert_eq!(login_page.status, ContextStatus::InProgress);
        }

        #[test]
        fn an_invalid_status_gives_a_ready_context() {
            let follow_up = import_fixtures().context(ACME, LOGIN_FOLLOW_UP);
            assert_eq!(follow_up.status, ContextStatus::Ready);
        }

        #[test]
        fn keeps_static_contexts_static() {
            let imported = import_fixtures();
            assert!(imported.context(ACME, RELEASE).is_static);
            assert!(!imported.context(ACME, DOCS).is_static);
        }
    }

    mod refused_contexts {
        use super::*;

        fn skipped_names(report: &ImportReport) -> Vec<(&str, &str)> {
            report
                .skipped_contexts
                .iter()
                .map(|skipped| (skipped.project.as_str(), skipped.context_name.as_str()))
                .collect()
        }

        #[test]
        fn keeps_the_project_without_the_refused_contexts() {
            let imported = import_fixtures();
            let project = imported.project(DUPLICATES);
            assert_eq!(project.contexts().len(), 1);
            assert_eq!(project.contexts()[0].name.as_str(), FEATURE_A);
        }

        #[test]
        fn reports_each_refused_context_with_its_reason() {
            let report = import_fixtures().report;
            assert_eq!(
                skipped_names(&report),
                [(DUPLICATES, "feature a"), (DUPLICATES, "Feature A!")]
            );
            assert_eq!(
                report.skipped_contexts[0].reason,
                "A context named 'feature a' already exists"
            );
            assert_eq!(
                report.skipped_contexts[1].reason,
                "A context already uses the folder 'feature-a'"
            );
        }
    }
}
