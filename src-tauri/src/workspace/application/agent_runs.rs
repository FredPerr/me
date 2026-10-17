//! The run-driving use case (#80). Running an agent is a workspace concern:
//! it picks the folder, seeds the permission floor, records the run and its
//! transcript, moves the context's status, captures the conversation id, and
//! streams a typed event the UI renders.
//!
//! The use case owns the orchestration; the pieces it drives are ports:
//! - an [`AgentRunner`] starts/stops the run and delivers its events to a sink,
//! - an [`AgentRunRepository`] persists the transcript (with the caps),
//! - a [`ProjectRepository`] holds the context's status and conversation id,
//! - [`AgentRunEvents`] emits the typed `agent-run` event,
//! - [`ProjectEvents`] tells open views the project changed on a status move,
//! - a [`Clock`] stamps the run's start, and a [`PermissionFloor`] seeds the
//!   deny policy.
//!
//! A run's events arrive on a background task after `run_agent` returns, so the
//! command can respond at once while the run streams. The service keeps the
//! [`RunHandle`] of each context's running run so `stop_agent_run` can find it;
//! the background task removes it when the run ends.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use tauri::async_runtime::JoinHandle;
use tokio::sync::mpsc;

use super::error::{AgentRunServiceError, WorkspaceServiceError};
use super::quit_guard::RunInProgress;
use crate::workspace::domain::agent_run::{AgentRun, AgentRunState};
use crate::workspace::domain::agent_runner::{
    AgentRunEvent, AgentRunEvents, AgentRunSpec, AgentRunUpdate, AgentRunner, RunFinalState,
    RunHandle,
};
use crate::workspace::domain::agent_work::AgentTool;
use crate::workspace::domain::context::Context;
use crate::workspace::domain::context_status::ContextStatus;
use crate::workspace::domain::error::WorkspaceError;
use crate::workspace::domain::identifiers::{AgentRunId, ContextId};
use crate::workspace::domain::ports::{
    AgentRunRepository, Clock, PermissionFloor, ProjectEvents, ProjectRepository,
};
use crate::workspace::domain::project::Project;
use crate::workspace::domain::project_tag::ProjectTag;

/// The Kiro adapter id, the only agent tool for now (spec #77, out of scope:
/// new tools). The adapter seam stays so adding one is later work.
const KIRO_TOOL_ID: &str = "kiro";

/// A run just started, returned so the command can report its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartedRun {
    pub run_id: AgentRunId,
    pub context_id: ContextId,
}

/// A context's in-flight run: the handle to stop it, the project and context
/// it belongs to (so the quit guard can name and stop it without re-reading
/// storage), and the background task draining its events. The consumer handle
/// lets a test await the run; in the app it only needs to exist to be replaced
/// when the run ends.
struct ActiveRun {
    handle: RunHandle,
    context_id: ContextId,
    context_name: String,
    #[cfg_attr(not(test), allow(dead_code))]
    consumer: JoinHandle<()>,
}

/// Drives agent runs for contexts of a project. Shares the project repository
/// with `WorkspaceService` and reuses its status/conversation changes so the
/// Project aggregate's rules (static contexts keep their status) are honoured.
pub struct AgentRunService {
    projects: Arc<dyn ProjectRepository>,
    agent_runs: Arc<dyn AgentRunRepository>,
    runner: Arc<dyn AgentRunner>,
    events: Arc<dyn AgentRunEvents>,
    project_events: Arc<dyn ProjectEvents>,
    clock: Arc<dyn Clock>,
    permission_floor: Arc<dyn PermissionFloor>,
    active: Mutex<HashMap<String, ActiveRun>>,
}

impl AgentRunService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        agent_runs: Arc<dyn AgentRunRepository>,
        runner: Arc<dyn AgentRunner>,
        events: Arc<dyn AgentRunEvents>,
        project_events: Arc<dyn ProjectEvents>,
        clock: Arc<dyn Clock>,
        permission_floor: Arc<dyn PermissionFloor>,
    ) -> Self {
        Self {
            projects,
            agent_runs,
            runner,
            events,
            project_events,
            clock,
            permission_floor,
            active: Mutex::new(HashMap::new()),
        }
    }

    /// Starts an agent on the context: picks the folder, seeds the permission
    /// floor, records a new `Running` run, moves a non-static context to in
    /// progress, and streams the run's events on a background task.
    pub async fn run_agent(
        self: &Arc<Self>,
        project_tag: &ProjectTag,
        context_id: &ContextId,
        prompt: String,
    ) -> Result<StartedRun, AgentRunServiceError> {
        let project = self.find_project(project_tag)?;
        let context = self.find_context(&project, context_id)?;
        let working_directory = project.ide_path(&context);
        self.permission_floor.seed(&working_directory);

        let spec = AgentRunSpec {
            tool_id: KIRO_TOOL_ID.to_string(),
            working_directory,
            prompt: prompt.clone(),
            resume_id: resume_id_of(&context),
        };

        let run = AgentRun::started(
            AgentRunId::generate(),
            context_id.clone(),
            prompt,
            self.clock.now_iso8601(),
        );
        self.agent_runs.start_run(&run)?;
        self.move_context(project_tag, &context, ContextStatus::InProgress)?;

        let (sink, events) = mpsc::unbounded_channel();
        let handle = self.runner.start(spec, sink).await?;

        let consumer = self.clone().spawn_consumer(
            project_tag.clone(),
            context.clone(),
            run.id.clone(),
            events,
        );
        self.active
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                context_id.as_str().to_string(),
                ActiveRun {
                    handle,
                    context_id: context_id.clone(),
                    context_name: context.name.to_string(),
                    consumer,
                },
            );
        Ok(StartedRun {
            run_id: run.id,
            context_id: context_id.clone(),
        })
    }

    /// Stops the context's running agent. Its sink then receives an `Ended`
    /// with a `Stopped` state, which the background task records like any end.
    pub async fn stop_agent_run(&self, context_id: &ContextId) -> Result<(), AgentRunServiceError> {
        let handle =
            self.active_handle(context_id)
                .ok_or_else(|| AgentRunServiceError::NoRunningAgent {
                    context: context_id.clone(),
                })?;
        self.runner.stop(&handle).await?;
        Ok(())
    }

    /// A context's recorded runs, oldest first, with their transcripts.
    pub fn list_agent_runs(
        &self,
        context_id: &ContextId,
    ) -> Result<Vec<AgentRun>, AgentRunServiceError> {
        Ok(self.agent_runs.list_runs(context_id)?)
    }

    /// The runs the current session is driving, each with its context's name,
    /// so the quit guard can decide whether to warn and what to say. The
    /// in-memory active handles are the source of truth: they are the runs this
    /// process actually started and can still stop, unlike opaque session ids.
    /// Ordered by context id for a stable message.
    pub fn in_progress_runs(&self) -> Vec<RunInProgress> {
        let active = self.active.lock().unwrap_or_else(PoisonError::into_inner);
        let mut runs: Vec<RunInProgress> = active
            .values()
            .map(|run| RunInProgress {
                context_id: run.context_id.clone(),
                context_name: run.context_name.clone(),
            })
            .collect();
        runs.sort_by(|left, right| left.context_id.as_str().cmp(right.context_id.as_str()));
        runs
    }

    /// Stops every run the current session is driving, for quitting cleanly.
    /// Each stop flows through the runner and, like `stop_agent_run`, its sink
    /// then delivers an `Ended(Stopped)` the background task records as Stopped
    /// while moving the context to needs human. Stops are attempted for all
    /// runs even if one fails; the first failure is returned afterwards.
    pub async fn stop_all_in_progress(&self) -> Result<(), AgentRunServiceError> {
        let handles: Vec<RunHandle> = self
            .active
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .map(|run| run.handle.clone())
            .collect();
        let mut first_error = None;
        for handle in handles {
            if let Err(error) = self.runner.stop(&handle).await {
                first_error.get_or_insert(AgentRunServiceError::from(error));
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Reconciles runs left `Running` in storage by a previous process that
    /// crashed or was force-quit (#82): each is recorded `Interrupted` and its
    /// context moved to needs human, unless the context is static. Called once
    /// at startup, before any run is started, so every `Running` run it finds
    /// is a leftover rather than one this process is driving.
    pub fn reconcile_interrupted_runs(&self) -> Result<(), AgentRunServiceError> {
        let interrupted = self.agent_runs.running_runs()?;
        for run in interrupted {
            self.agent_runs.finish_run(
                &run.context_id,
                &run.id,
                AgentRunState::Interrupted,
                None,
            )?;
            self.move_interrupted_context(&run.context_id)?;
        }
        Ok(())
    }

    /// Moves the context of an interrupted run to needs human. The owning
    /// project is found from storage because a leftover run only knows its
    /// context id; a static context keeps its status through `move_context`.
    fn move_interrupted_context(&self, context_id: &ContextId) -> Result<(), AgentRunServiceError> {
        let Some((project_tag, context)) = self.find_owning_project(context_id)? else {
            return Ok(());
        };
        self.move_context(&project_tag, &context, ContextStatus::NeedsHuman)
    }

    /// The project tag and context owning `context_id`, searched across all
    /// readable projects. `None` when no project holds the context (its project
    /// may have been deleted since the run was recorded).
    fn find_owning_project(
        &self,
        context_id: &ContextId,
    ) -> Result<Option<(ProjectTag, Context)>, AgentRunServiceError> {
        let listing = self.projects.list().map_err(WorkspaceServiceError::from)?;
        Ok(listing.projects.into_iter().find_map(|project| {
            project
                .find_context(context_id)
                .map(|context| (project.tag().clone(), context.clone()))
        }))
    }

    /// Consumes a run's events on a background task: appends lines, records the
    /// conversation id once, ends the run and moves the context when it ends,
    /// and emits the typed event throughout.
    fn spawn_consumer(
        self: Arc<Self>,
        project_tag: ProjectTag,
        context: Context,
        run_id: AgentRunId,
        mut events: mpsc::UnboundedReceiver<AgentRunEvent>,
    ) -> JoinHandle<()> {
        tauri::async_runtime::spawn(async move {
            let context_id = context.id.clone();
            let mut conversation_id_recorded = false;
            // Stop on `Ended`, not when the channel closes: a run has one
            // terminal event and no more follow it. The runner may keep its
            // sink alive, so waiting for the channel to close would hang.
            while let Some(event) = events.recv().await {
                if self
                    .handle_event(
                        &project_tag,
                        &context,
                        &run_id,
                        event,
                        &mut conversation_id_recorded,
                    )
                    .is_break()
                {
                    break;
                }
            }
            self.active
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(context_id.as_str());
        })
    }

    fn handle_event(
        &self,
        project_tag: &ProjectTag,
        context: &Context,
        run_id: &AgentRunId,
        event: AgentRunEvent,
        conversation_id_recorded: &mut bool,
    ) -> std::ops::ControlFlow<()> {
        let context_id = &context.id;
        match event {
            AgentRunEvent::Started { .. } => {
                self.emit(context_id, run_id, AgentRunUpdate::Started);
                std::ops::ControlFlow::Continue(())
            }
            AgentRunEvent::Line { text } => {
                self.append_line(context_id, run_id, &text);
                self.emit(context_id, run_id, AgentRunUpdate::Line { text });
                std::ops::ControlFlow::Continue(())
            }
            AgentRunEvent::ConversationId { id } => {
                if !*conversation_id_recorded {
                    *conversation_id_recorded = true;
                    self.record_conversation(project_tag, context_id, id);
                }
                std::ops::ControlFlow::Continue(())
            }
            AgentRunEvent::Ended {
                final_state,
                exit_code,
            } => {
                self.finish_run(context_id, run_id, final_state, exit_code);
                let _ = self.move_context(project_tag, context, ContextStatus::NeedsHuman);
                self.emit(
                    context_id,
                    run_id,
                    AgentRunUpdate::Ended {
                        final_state,
                        exit_code,
                    },
                );
                std::ops::ControlFlow::Break(())
            }
        }
    }

    fn append_line(&self, context_id: &ContextId, run_id: &AgentRunId, text: &str) {
        if let Err(error) = self.agent_runs.append_lines(
            context_id,
            run_id,
            std::slice::from_ref(&text.to_string()),
        ) {
            eprintln!("[workspace] agent-run line for '{context_id}' not stored: {error}");
        }
    }

    fn finish_run(
        &self,
        context_id: &ContextId,
        run_id: &AgentRunId,
        final_state: RunFinalState,
        exit_code: Option<i32>,
    ) {
        if let Err(error) =
            self.agent_runs
                .finish_run(context_id, run_id, state_of(final_state), exit_code)
        {
            eprintln!("[workspace] agent-run end for '{context_id}' not stored: {error}");
        }
    }

    fn record_conversation(&self, project_tag: &ProjectTag, context_id: &ContextId, id: String) {
        let context_id = context_id.clone();
        let change = move |project: Project| project.record_agent_conversation(&context_id, id);
        match self.projects.update(project_tag, Box::new(change)) {
            Ok(project) => self.project_events.project_changed(project.tag()),
            Err(error) => eprintln!("[workspace] agent conversation not recorded: {error}"),
        }
    }

    /// Moves the context's status through the Project aggregate, which refuses
    /// a static context. A static context keeps its status: the refusal is
    /// expected and ignored, matching the old frontend's `moveContext` guard.
    fn move_context(
        &self,
        project_tag: &ProjectTag,
        context: &Context,
        status: ContextStatus,
    ) -> Result<(), AgentRunServiceError> {
        if context.is_static {
            return Ok(());
        }
        let context_id = context.id.clone();
        let saved = self.projects.update(
            project_tag,
            Box::new(move |project| project.set_status(&context_id, status)),
        )?;
        self.project_events.project_changed(saved.tag());
        Ok(())
    }

    fn emit(&self, context_id: &ContextId, run_id: &AgentRunId, update: AgentRunUpdate) {
        self.events
            .run_changed(context_id.as_str(), run_id.as_str(), update);
    }

    fn find_project(&self, tag: &ProjectTag) -> Result<Project, AgentRunServiceError> {
        self.projects
            .find(tag)?
            .ok_or_else(|| WorkspaceServiceError::ProjectNotFound { tag: tag.clone() }.into())
    }

    fn find_context(
        &self,
        project: &Project,
        context_id: &ContextId,
    ) -> Result<Context, AgentRunServiceError> {
        project.find_context(context_id).cloned().ok_or_else(|| {
            WorkspaceServiceError::Rejected(WorkspaceError::UnknownContext {
                id: context_id.clone(),
            })
            .into()
        })
    }

    fn active_handle(&self, context_id: &ContextId) -> Option<RunHandle> {
        self.active
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(context_id.as_str())
            .map(|active| active.handle.clone())
    }

    /// Takes the background consumer task of the context's run so a test can
    /// await it, leaving a finished placeholder in its place.
    #[cfg(test)]
    fn take_consumer(&self, context_id: &ContextId) -> Option<JoinHandle<()>> {
        self.active
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(context_id.as_str())
            .map(|run| std::mem::replace(&mut run.consumer, tauri::async_runtime::spawn(async {})))
    }
}

/// The context's stored Kiro conversation id, to resume the same conversation.
fn resume_id_of(context: &Context) -> Option<String> {
    let agent = context.agent.as_ref()?;
    match &agent.tool {
        AgentTool::Kiro { conversation_id } => conversation_id.clone(),
    }
}

fn state_of(final_state: RunFinalState) -> AgentRunState {
    match final_state {
        RunFinalState::Finished => AgentRunState::Finished,
        RunFinalState::Failed => AgentRunState::Failed,
        RunFinalState::Stopped => AgentRunState::Stopped,
    }
}

#[cfg(test)]
#[path = "agent_runs_tests.rs"]
mod tests;
