//! Recording fakes of the checkout files and project events ports for the
//! service tests. Files are modelled as a set of symlinks; any symlink can be
//! told to fail.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::workspace::domain::agent_runner::{
    AgentRunError, AgentRunEvent, AgentRunEventSink, AgentRunEvents, AgentRunSpec, AgentRunUpdate,
    AgentRunner, RunFinalState, RunHandle,
};
use crate::workspace::domain::agent_work::AgentTool;
use crate::workspace::domain::ports::{
    CheckoutFiles, CheckoutFilesError, Clock, DefaultAgentTool, PermissionFloor, ProjectEvents,
    WorkbenchEvents,
};
use crate::workspace::domain::project_tag::ProjectTag;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A post-checkout command run in a checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostCheckoutRun {
    pub command: String,
    pub checkout: PathBuf,
}

#[derive(Default)]
struct FilesState {
    symlinks: BTreeSet<PathBuf>,
    removed_folders: Vec<PathBuf>,
    post_checkout_runs: Vec<PostCheckoutRun>,
    failing_links: Vec<PathBuf>,
}

#[derive(Default)]
pub struct RecordingCheckoutFiles {
    state: Mutex<FilesState>,
}

impl RecordingCheckoutFiles {
    pub fn symlinks(&self) -> Vec<PathBuf> {
        lock(&self.state).symlinks.iter().cloned().collect()
    }

    pub fn post_checkout_runs(&self) -> Vec<PostCheckoutRun> {
        lock(&self.state).post_checkout_runs.clone()
    }

    pub fn removed_folders(&self) -> Vec<PathBuf> {
        lock(&self.state).removed_folders.clone()
    }

    /// A symlink already on disk, such as a legacy linked repository.
    pub fn add_symlink(&self, link: &str) {
        lock(&self.state).symlinks.insert(PathBuf::from(link));
    }

    /// Creating a symlink at `link` fails from now on.
    pub fn fail_on_link(&self, link: &str) {
        lock(&self.state).failing_links.push(PathBuf::from(link));
    }
}

impl CheckoutFiles for RecordingCheckoutFiles {
    fn create_symlink(&self, _target: &Path, link: &Path) -> Result<bool, CheckoutFilesError> {
        let mut state = lock(&self.state);
        if state.failing_links.iter().any(|failing| failing == link) {
            return Err(CheckoutFilesError {
                path: link.display().to_string(),
                reason: "permission denied".to_string(),
            });
        }
        Ok(state.symlinks.insert(link.to_path_buf()))
    }

    fn remove_symlink(&self, link: &Path) -> Result<(), CheckoutFilesError> {
        lock(&self.state).symlinks.remove(link);
        Ok(())
    }

    fn remove_folder_if_empty(&self, folder: &Path) {
        lock(&self.state).removed_folders.push(folder.to_path_buf());
    }

    fn run_post_checkout(&self, command: &str, checkout: &Path) {
        lock(&self.state).post_checkout_runs.push(PostCheckoutRun {
            command: command.to_string(),
            checkout: checkout.to_path_buf(),
        });
    }
}

#[derive(Default)]
pub struct RecordingProjectEvents {
    changed: Mutex<Vec<ProjectTag>>,
}

impl RecordingProjectEvents {
    pub fn changed_projects(&self) -> Vec<ProjectTag> {
        lock(&self.changed).clone()
    }
}

impl ProjectEvents for RecordingProjectEvents {
    fn project_changed(&self, tag: &ProjectTag) {
        lock(&self.changed).push(tag.clone());
    }
}

/// Counts the `WorkbenchChanged` signals the service emits, so a test can prove
/// a save emitted the change. The signal carries no payload, so a count is
/// enough.
#[derive(Default)]
pub struct RecordingWorkbenchEvents {
    changes: Mutex<usize>,
}

impl RecordingWorkbenchEvents {
    pub fn change_count(&self) -> usize {
        *lock(&self.changes)
    }
}

impl WorkbenchEvents for RecordingWorkbenchEvents {
    fn workbench_changed(&self) {
        *lock(&self.changes) += 1;
    }
}

/// One `agent-run` update the use case emitted, with the run it concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedAgentRunUpdate {
    pub context_id: String,
    pub run_id: String,
    pub update: AgentRunUpdate,
}

/// Records the `agent-run` events the use case emits, in order.
#[derive(Default)]
pub struct RecordingAgentRunEvents {
    updates: Mutex<Vec<RecordedAgentRunUpdate>>,
}

impl RecordingAgentRunEvents {
    pub fn updates(&self) -> Vec<RecordedAgentRunUpdate> {
        lock(&self.updates).clone()
    }
}

impl AgentRunEvents for RecordingAgentRunEvents {
    fn run_changed(&self, context_id: &str, run_id: &str, update: AgentRunUpdate) {
        lock(&self.updates).push(RecordedAgentRunUpdate {
            context_id: context_id.to_string(),
            run_id: run_id.to_string(),
            update,
        });
    }
}

/// A clock fixed at one ISO 8601 instant, so a run's `started_at` is known.
pub struct FakeClock {
    now: Mutex<String>,
}

impl FakeClock {
    pub fn at(now: &str) -> Self {
        Self {
            now: Mutex::new(now.to_string()),
        }
    }
}

impl Clock for FakeClock {
    fn now_iso8601(&self) -> String {
        lock(&self.now).clone()
    }
}

/// Records the working directories the use case asked to seed, without
/// touching the disk.
#[derive(Default)]
pub struct RecordingPermissionFloor {
    seeded: Mutex<Vec<String>>,
}

impl RecordingPermissionFloor {
    pub fn seeded(&self) -> Vec<String> {
        lock(&self.seeded).clone()
    }
}

impl PermissionFloor for RecordingPermissionFloor {
    fn seed(&self, working_directory: &str) {
        lock(&self.seeded).push(working_directory.to_string());
    }
}

/// A fake Default-AI-tool port returning a fixed tool and recording whether it
/// was consulted, so a test can prove a fresh context reads the default while
/// an established one does not.
pub struct FakeDefaultAgentTool {
    tool: AgentTool,
    consulted: Mutex<bool>,
}

impl FakeDefaultAgentTool {
    pub fn returning(tool: AgentTool) -> Self {
        Self {
            tool,
            consulted: Mutex::new(false),
        }
    }

    pub fn was_consulted(&self) -> bool {
        *lock(&self.consulted)
    }
}

impl DefaultAgentTool for FakeDefaultAgentTool {
    fn default_tool(&self) -> AgentTool {
        *lock(&self.consulted) = true;
        self.tool.clone()
    }
}

/// A fake `AgentRunner` for the service tests: it delivers a scripted list of
/// events through the sink when `start` is called, records the spec it was
/// started with, and lets a test ask it to deliver a terminal `Ended` event
/// later to stand in for a process ending. No real process is spawned.
pub struct FakeAgentRunner {
    run_id: String,
    scripted: Mutex<Vec<AgentRunEvent>>,
    started_specs: Mutex<Vec<AgentRunSpec>>,
    stopped: Mutex<Vec<RunHandle>>,
    /// The sink of the most recent run, so a test can push a later `Ended`.
    sink: Mutex<Option<AgentRunEventSink>>,
    /// An error to return from `start` instead of starting, when set.
    start_error: Mutex<Option<AgentRunError>>,
}

impl FakeAgentRunner {
    /// A runner that, on `start`, delivers `scripted` (after a `Started`) and
    /// uses `run_id` as the handle.
    pub fn scripted(run_id: &str, scripted: Vec<AgentRunEvent>) -> Self {
        Self {
            run_id: run_id.to_string(),
            scripted: Mutex::new(scripted),
            started_specs: Mutex::new(Vec::new()),
            stopped: Mutex::new(Vec::new()),
            sink: Mutex::new(None),
            start_error: Mutex::new(None),
        }
    }

    pub fn started_specs(&self) -> Vec<AgentRunSpec> {
        lock(&self.started_specs).clone()
    }

    pub fn stopped_handles(&self) -> Vec<RunHandle> {
        lock(&self.stopped).clone()
    }

    /// Delivers a terminal `Ended` event through the latest run's sink, as the
    /// underlying process ending would. Returns false when no run is active.
    pub fn end_latest_run(&self, final_state: RunFinalState, exit_code: Option<i32>) -> bool {
        match lock(&self.sink).as_ref() {
            Some(sink) => sink
                .send(AgentRunEvent::Ended {
                    final_state,
                    exit_code,
                })
                .is_ok(),
            None => false,
        }
    }
}

#[async_trait::async_trait]
impl AgentRunner for FakeAgentRunner {
    async fn start(
        &self,
        spec: AgentRunSpec,
        sink: AgentRunEventSink,
    ) -> Result<RunHandle, AgentRunError> {
        if let Some(error) = lock(&self.start_error).take() {
            return Err(error);
        }
        lock(&self.started_specs).push(spec);
        let _ = sink.send(AgentRunEvent::Started {
            run_id: self.run_id.clone(),
        });
        for event in lock(&self.scripted).drain(..) {
            let _ = sink.send(event);
        }
        *lock(&self.sink) = Some(sink);
        Ok(RunHandle {
            run_id: self.run_id.clone(),
        })
    }

    async fn stop(&self, handle: &RunHandle) -> Result<(), AgentRunError> {
        lock(&self.stopped).push(handle.clone());
        Ok(())
    }
}
