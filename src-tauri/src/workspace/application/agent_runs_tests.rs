//! Seam-A tests of the run-driving use case against fakes: an in-memory
//! project repository and agent-run store, a fake `AgentRunner` that emits
//! scripted events through the sink, a fixed clock, a recording permission
//! floor and recording events. No real process is spawned.

use super::*;
use crate::workspace::domain::agent_run::AgentRunState;
use crate::workspace::domain::agent_runner::{AgentRunEvent, RunFinalState};
use crate::workspace::domain::agent_work::{AgentTool, AgentWork};
use crate::workspace::domain::context::Context;
use crate::workspace::domain::context_name::ContextName;
use crate::workspace::domain::context_status::ContextStatus;
use crate::workspace::domain::project::ProjectSettings;
use crate::workspace::domain::test_support::{settings, PROJECT_TAG};
use crate::workspace::infra::in_memory_agent_run_repository::InMemoryAgentRunRepository;
use crate::workspace::infra::in_memory_project_repository::InMemoryProjectRepository;
use crate::workspace::infra::recording_fakes::{
    FakeAgentRunner, FakeClock, FakeDefaultAgentTool, RecordedAgentRunUpdate,
    RecordingAgentRunEvents, RecordingPermissionFloor, RecordingProjectEvents,
};

const FEATURE_CONTEXT: &str = "Login page";
const PROMPT: &str = "Build the login page";
const RUN_ID: &str = "run-1";
const STARTED_AT: &str = "2024-01-15T10:00:00Z";
const CONVERSATION_ID: &str = "conv-123";
const FIRST_LINE: &str = "compiling";
const SECOND_LINE: &str = "done";
const EXISTING_CONVERSATION: &str = "conv-prior";

fn tag() -> ProjectTag {
    ProjectTag::parse(PROJECT_TAG).unwrap()
}

fn prompt() -> String {
    PROMPT.to_string()
}

/// A manual (non-static) feature context, optionally with a stored
/// conversation id to resume.
fn feature_context(conversation_id: Option<&str>) -> Context {
    let agent = conversation_id.map(|id| AgentWork::default().with_conversation(id.to_string()));
    Context {
        agent,
        ..Context::new(ContextName::parse(FEATURE_CONTEXT).unwrap(), vec![]).unwrap()
    }
}

/// The tool the harness's default-tool port hands a fresh context, unless a
/// test overrides it. Kiro, the only tool today, so existing assertions stand.
fn default_kiro_tool() -> AgentTool {
    AgentTool::Kiro {
        conversation_id: None,
    }
}

struct Harness {
    projects: Arc<InMemoryProjectRepository>,
    agent_runs: Arc<InMemoryAgentRunRepository>,
    runner: Arc<FakeAgentRunner>,
    events: Arc<RecordingAgentRunEvents>,
    permission_floor: Arc<RecordingPermissionFloor>,
    default_tool: Arc<FakeDefaultAgentTool>,
    service: Arc<AgentRunService>,
    context_id: ContextId,
}

impl Harness {
    /// A project with a default (static) context and `feature_context`, a
    /// runner scripted to emit `scripted` after `Started`, and the default-tool
    /// port returning `default_tool` for a fresh context.
    fn with_default_tool(
        context: Context,
        scripted: Vec<AgentRunEvent>,
        default_tool: AgentTool,
    ) -> Self {
        let context_id = context.id.clone();
        let project = Project::new(tag(), feature_settings())
            .add_context(Context::new_default(vec![]))
            .unwrap()
            .add_context(context)
            .unwrap();
        let projects = Arc::new(InMemoryProjectRepository::default());
        projects.save(&project).unwrap();

        let agent_runs = Arc::new(InMemoryAgentRunRepository::default());
        let runner = Arc::new(FakeAgentRunner::scripted(RUN_ID, scripted));
        let events = Arc::new(RecordingAgentRunEvents::default());
        let permission_floor = Arc::new(RecordingPermissionFloor::default());
        let default_tool = Arc::new(FakeDefaultAgentTool::returning(default_tool));
        let service = Arc::new(AgentRunService::new(
            projects.clone(),
            agent_runs.clone(),
            runner.clone(),
            events.clone(),
            Arc::new(RecordingProjectEvents::default()),
            Arc::new(FakeClock::at(STARTED_AT)),
            permission_floor.clone(),
            default_tool.clone(),
        ));
        Self {
            projects,
            agent_runs,
            runner,
            events,
            permission_floor,
            default_tool,
            service,
            context_id,
        }
    }

    /// A harness whose default-tool port hands a fresh context Kiro.
    fn with(context: Context, scripted: Vec<AgentRunEvent>) -> Self {
        Self::with_default_tool(context, scripted, default_kiro_tool())
    }

    fn feature(scripted: Vec<AgentRunEvent>) -> Self {
        Self::with(feature_context(None), scripted)
    }

    async fn run(&self) -> StartedRun {
        self.service
            .run_agent(&tag(), &self.context_id, prompt())
            .await
            .unwrap()
    }

    /// Runs and awaits the background consumer so every event is applied.
    async fn run_to_completion(&self) -> StartedRun {
        let started = self.run().await;
        self.await_consumer().await;
        started
    }

    async fn await_consumer(&self) {
        if let Some(consumer) = self.service.take_consumer(&self.context_id) {
            consumer.await.unwrap();
        }
    }

    fn status(&self) -> ContextStatus {
        self.stored_context().status
    }

    fn stored_context(&self) -> Context {
        self.projects
            .find(&tag())
            .unwrap()
            .unwrap()
            .find_context(&self.context_id)
            .unwrap()
            .clone()
    }

    fn only_run(&self) -> AgentRun {
        let runs = self.agent_runs.list_runs(&self.context_id).unwrap();
        assert_eq!(runs.len(), 1, "expected a single recorded run");
        runs.into_iter().next().unwrap()
    }

    fn updates(&self) -> Vec<RecordedAgentRunUpdate> {
        self.events.updates()
    }
}

fn feature_settings() -> ProjectSettings {
    settings(vec![])
}

fn line(text: &str) -> AgentRunEvent {
    AgentRunEvent::Line {
        text: text.to_string(),
    }
}

fn ended(final_state: RunFinalState) -> AgentRunEvent {
    AgentRunEvent::Ended {
        final_state,
        exit_code: Some(0),
    }
}

mod starting {
    use super::*;

    #[tokio::test]
    async fn moves_a_non_static_context_to_in_progress() {
        let harness = Harness::feature(vec![]);

        harness.run().await;

        assert_eq!(harness.status(), ContextStatus::InProgress);
    }

    #[tokio::test]
    async fn records_a_running_run_with_the_prompt_and_start_time() {
        let harness = Harness::feature(vec![]);

        let started = harness.run().await;

        let run = harness.only_run();
        assert_eq!(run.id, started.run_id);
        assert_eq!(run.prompt, PROMPT);
        assert_eq!(run.started_at, STARTED_AT);
        assert_eq!(run.state, AgentRunState::Running);
    }

    #[tokio::test]
    async fn seeds_the_permission_floor_in_the_run_folder() {
        let harness = Harness::feature(vec![]);
        let folder = harness.stored_context();
        let project = harness.projects.find(&tag()).unwrap().unwrap();

        harness.run().await;

        assert_eq!(
            harness.permission_floor.seeded(),
            [project.ide_path(&folder)]
        );
    }

    #[tokio::test]
    async fn starts_the_runner_for_kiro_in_the_run_folder() {
        let harness = Harness::feature(vec![]);
        let project = harness.projects.find(&tag()).unwrap().unwrap();
        let expected_folder = project.ide_path(&harness.stored_context());

        harness.run().await;

        let spec = harness.runner.started_specs().into_iter().next().unwrap();
        assert_eq!(spec.tool_id, "kiro");
        assert_eq!(spec.working_directory, expected_folder);
        assert_eq!(spec.prompt, PROMPT);
    }

    #[tokio::test]
    async fn resumes_the_stored_conversation_when_present() {
        let harness = Harness::with(feature_context(Some(EXISTING_CONVERSATION)), vec![]);

        harness.run().await;

        let spec = harness.runner.started_specs().into_iter().next().unwrap();
        assert_eq!(spec.resume_id.as_deref(), Some(EXISTING_CONVERSATION));
    }

    #[tokio::test]
    async fn reports_an_unknown_context() {
        let harness = Harness::feature(vec![]);
        let unknown = ContextId::generate();

        let result = harness.service.run_agent(&tag(), &unknown, prompt()).await;

        assert!(matches!(
            result,
            Err(AgentRunServiceError::Workspace(
                WorkspaceServiceError::Rejected(WorkspaceError::UnknownContext { .. })
            ))
        ));
    }

    #[tokio::test]
    async fn reports_an_unknown_project() {
        let harness = Harness::feature(vec![]);
        let other = ProjectTag::parse("other").unwrap();

        let result = harness
            .service
            .run_agent(&other, &harness.context_id, prompt())
            .await;

        assert!(matches!(
            result,
            Err(AgentRunServiceError::Workspace(
                WorkspaceServiceError::ProjectNotFound { .. }
            ))
        ));
    }
}

mod a_static_context {
    use super::*;

    fn static_harness(scripted: Vec<AgentRunEvent>) -> Harness {
        let context = Context {
            is_static: true,
            ..feature_context(None)
        };
        Harness::with(context, scripted)
    }

    #[tokio::test]
    async fn keeps_its_status_but_still_records_the_run() {
        let harness = static_harness(vec![]);
        let status_before = harness.status();

        harness.run().await;

        assert_eq!(harness.status(), status_before);
        assert_eq!(harness.only_run().state, AgentRunState::Running);
    }

    #[tokio::test]
    async fn is_left_alone_when_the_run_ends() {
        let harness = static_harness(vec![ended(RunFinalState::Finished)]);
        let status_before = harness.status();

        harness.run_to_completion().await;

        assert_eq!(harness.status(), status_before);
        assert_eq!(harness.only_run().state, AgentRunState::Finished);
    }

    #[tokio::test]
    async fn still_streams_its_events() {
        let harness = static_harness(vec![line(FIRST_LINE), ended(RunFinalState::Finished)]);

        harness.run_to_completion().await;

        let kinds: Vec<AgentRunUpdate> = harness.updates().into_iter().map(|u| u.update).collect();
        assert_eq!(
            kinds,
            vec![
                AgentRunUpdate::Started,
                AgentRunUpdate::Line {
                    text: FIRST_LINE.to_string()
                },
                AgentRunUpdate::Ended {
                    final_state: RunFinalState::Finished,
                    exit_code: Some(0),
                },
            ]
        );
    }
}

mod ending {
    use super::*;

    async fn ends_with(final_state: RunFinalState) -> Harness {
        let harness = Harness::feature(vec![ended(final_state)]);
        harness.run_to_completion().await;
        harness
    }

    #[tokio::test]
    async fn a_finished_run_moves_the_context_to_needs_human() {
        let harness = ends_with(RunFinalState::Finished).await;

        assert_eq!(harness.status(), ContextStatus::NeedsHuman);
        assert_eq!(harness.only_run().state, AgentRunState::Finished);
    }

    #[tokio::test]
    async fn a_failed_run_moves_the_context_to_needs_human() {
        let harness = ends_with(RunFinalState::Failed).await;

        assert_eq!(harness.status(), ContextStatus::NeedsHuman);
        assert_eq!(harness.only_run().state, AgentRunState::Failed);
    }

    #[tokio::test]
    async fn a_stopped_run_moves_the_context_to_needs_human() {
        let harness = ends_with(RunFinalState::Stopped).await;

        assert_eq!(harness.status(), ContextStatus::NeedsHuman);
        assert_eq!(harness.only_run().state, AgentRunState::Stopped);
    }

    #[tokio::test]
    async fn the_exit_code_is_recorded() {
        let harness = Harness::feature(vec![AgentRunEvent::Ended {
            final_state: RunFinalState::Failed,
            exit_code: Some(2),
        }]);

        harness.run_to_completion().await;

        assert_eq!(harness.only_run().exit_code, Some(2));
    }
}

mod transcript {
    use super::*;

    #[tokio::test]
    async fn lines_are_persisted_in_order() {
        let harness = Harness::feature(vec![
            line(FIRST_LINE),
            line(SECOND_LINE),
            ended(RunFinalState::Finished),
        ]);

        harness.run_to_completion().await;

        assert_eq!(
            harness.only_run().output_lines,
            vec![FIRST_LINE, SECOND_LINE]
        );
    }
}

mod resolving_the_tool {
    use super::*;

    #[tokio::test]
    async fn a_fresh_context_reads_the_default_tool_from_the_port() {
        let harness = Harness::with_default_tool(
            feature_context(None),
            vec![],
            AgentTool::Kiro {
                conversation_id: None,
            },
        );

        harness.run().await;

        assert!(
            harness.default_tool.was_consulted(),
            "a fresh context must consult the default-tool port"
        );
        let spec = harness.runner.started_specs().into_iter().next().unwrap();
        assert_eq!(spec.tool_id, "kiro");
    }

    #[tokio::test]
    async fn a_fresh_context_records_the_tool_it_started_on() {
        let harness = Harness::feature(vec![]);

        harness.run().await;

        let agent = harness.stored_context().agent.expect("agent work recorded");
        assert_eq!(
            agent.tool,
            AgentTool::Kiro {
                conversation_id: None
            }
        );
    }

    #[tokio::test]
    async fn an_established_context_keeps_its_recorded_tool_without_the_port() {
        let harness = Harness::with(feature_context(Some(EXISTING_CONVERSATION)), vec![]);

        harness.run().await;

        assert!(
            !harness.default_tool.was_consulted(),
            "a context with recorded agent work must not consult the port"
        );
        let spec = harness.runner.started_specs().into_iter().next().unwrap();
        assert_eq!(spec.tool_id, "kiro");
        // The recorded tool wins, so its conversation id is resumed.
        assert_eq!(spec.resume_id.as_deref(), Some(EXISTING_CONVERSATION));
    }
}

mod conversation_id {
    use super::*;

    fn conversation(id: &str) -> AgentRunEvent {
        AgentRunEvent::ConversationId { id: id.to_string() }
    }

    #[tokio::test]
    async fn the_first_is_saved_once_into_agent_work() {
        let harness = Harness::feature(vec![
            conversation(CONVERSATION_ID),
            conversation("conv-later"),
            ended(RunFinalState::Finished),
        ]);

        harness.run_to_completion().await;

        let agent = harness.stored_context().agent.unwrap();
        assert_eq!(
            agent.tool,
            AgentTool::Kiro {
                conversation_id: Some(CONVERSATION_ID.to_string())
            }
        );
    }
}

mod stopping {
    use super::*;

    #[tokio::test]
    async fn stops_the_running_agent_and_it_ends_stopped() {
        let harness = Harness::feature(vec![]);
        harness.run().await;

        harness
            .service
            .stop_agent_run(&harness.context_id)
            .await
            .unwrap();
        // The process ending after a stop flows through the same sink.
        harness.runner.end_latest_run(RunFinalState::Stopped, None);
        harness.await_consumer().await;

        assert_eq!(harness.runner.stopped_handles().len(), 1);
        assert_eq!(harness.only_run().state, AgentRunState::Stopped);
        assert_eq!(harness.status(), ContextStatus::NeedsHuman);
    }

    #[tokio::test]
    async fn reports_no_running_agent_for_an_idle_context() {
        let harness = Harness::feature(vec![]);

        let result = harness.service.stop_agent_run(&harness.context_id).await;

        assert!(matches!(
            result,
            Err(AgentRunServiceError::NoRunningAgent { .. })
        ));
    }
}

mod in_progress_runs {
    use super::*;

    #[tokio::test]
    async fn lists_a_live_run_with_its_context_name() {
        let harness = Harness::feature(vec![]);
        harness.run().await;

        let in_progress = harness.service.in_progress_runs();

        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].context_id, harness.context_id);
        assert_eq!(in_progress[0].context_name, FEATURE_CONTEXT);
    }

    #[tokio::test]
    async fn is_empty_when_no_run_is_active() {
        let harness = Harness::feature(vec![]);

        assert!(harness.service.in_progress_runs().is_empty());
    }

    #[tokio::test]
    async fn is_empty_again_once_the_run_ends() {
        let harness = Harness::feature(vec![ended(RunFinalState::Finished)]);

        harness.run_to_completion().await;

        assert!(harness.service.in_progress_runs().is_empty());
    }
}

mod stop_all_in_progress {
    use super::*;

    #[tokio::test]
    async fn stops_each_run_and_records_it_stopped_with_the_context_needing_human() {
        let harness = Harness::feature(vec![]);
        harness.run().await;

        harness.service.stop_all_in_progress().await.unwrap();
        // The process ending after the stop flows through the same sink.
        harness.runner.end_latest_run(RunFinalState::Stopped, None);
        harness.await_consumer().await;

        assert_eq!(harness.runner.stopped_handles().len(), 1);
        assert_eq!(harness.only_run().state, AgentRunState::Stopped);
        assert_eq!(harness.status(), ContextStatus::NeedsHuman);
    }

    #[tokio::test]
    async fn does_nothing_when_no_run_is_active() {
        let harness = Harness::feature(vec![]);

        harness.service.stop_all_in_progress().await.unwrap();

        assert!(harness.runner.stopped_handles().is_empty());
    }
}

mod reconciling_interrupted_runs {
    use super::*;
    use crate::workspace::domain::agent_run::AgentRun;

    const LEFTOVER_RUN: &str = "leftover-run";

    /// Persists a run left `Running` on the harness's context, as a crashed
    /// process would, without going through `run_agent`.
    fn persist_running_run(harness: &Harness) {
        let run = AgentRun::started(
            AgentRunId::parse(LEFTOVER_RUN).unwrap(),
            harness.context_id.clone(),
            PROMPT.to_string(),
            STARTED_AT.to_string(),
        );
        harness.agent_runs.start_run(&run).unwrap();
    }

    #[tokio::test]
    async fn a_leftover_run_becomes_interrupted_and_its_context_needs_human() {
        let harness = Harness::feature(vec![]);
        persist_running_run(&harness);

        harness.service.reconcile_interrupted_runs().unwrap();

        assert_eq!(harness.only_run().state, AgentRunState::Interrupted);
        assert_eq!(harness.status(), ContextStatus::NeedsHuman);
    }

    #[tokio::test]
    async fn a_static_context_keeps_its_status_but_its_run_is_still_interrupted() {
        let context = Context {
            is_static: true,
            ..feature_context(None)
        };
        let harness = Harness::with(context, vec![]);
        let status_before = harness.status();
        persist_running_run(&harness);

        harness.service.reconcile_interrupted_runs().unwrap();

        assert_eq!(harness.status(), status_before);
        assert_eq!(harness.only_run().state, AgentRunState::Interrupted);
    }

    #[tokio::test]
    async fn does_nothing_when_no_run_was_left_running() {
        let harness = Harness::feature(vec![]);

        harness.service.reconcile_interrupted_runs().unwrap();

        assert!(harness
            .agent_runs
            .list_runs(&harness.context_id)
            .unwrap()
            .is_empty());
        assert_eq!(harness.status(), ContextStatus::default());
    }
}

mod the_typed_event {
    use super::*;

    #[tokio::test]
    async fn is_emitted_for_started_line_and_ended_with_the_context_and_run() {
        let harness = Harness::feature(vec![line(FIRST_LINE), ended(RunFinalState::Finished)]);

        let started = harness.run_to_completion().await;

        let updates = harness.updates();
        assert!(updates
            .iter()
            .all(|u| u.context_id == harness.context_id.as_str()
                && u.run_id == started.run_id.as_str()));
        let kinds: Vec<AgentRunUpdate> = updates.into_iter().map(|u| u.update).collect();
        assert_eq!(
            kinds,
            vec![
                AgentRunUpdate::Started,
                AgentRunUpdate::Line {
                    text: FIRST_LINE.to_string()
                },
                AgentRunUpdate::Ended {
                    final_state: RunFinalState::Finished,
                    exit_code: Some(0),
                },
            ]
        );
    }
}
