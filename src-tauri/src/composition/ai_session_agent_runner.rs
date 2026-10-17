//! `workspace`'s `AgentRunner`, run by `ai_session`. The only place the two
//! modules meet: `workspace` depends on the port, `ai_session` on the process
//! runner, and this adapter bridges them.
//!
//! It resolves the tool's adapter, builds the spawn spec, spawns the child with
//! an in-process sink, and forwards the runner's lines and terminal status to
//! the caller's `AgentRunEvent` sink. The conversation id is read from the
//! output lines through the adapter's own `conversation_id`, once per run.
//! `ai_session`'s existing Tauri events still fire, so the frontend is
//! unaffected until it is swapped (#81).

use async_trait::async_trait;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use crate::ai_session::adapter::{CliAdapter, SpawnRequest};
use crate::ai_session::process::{self, AiSessionEvent};
use crate::ai_session::registry::{SessionRegistry, SessionStatus};
use crate::ai_session::OutputStream;
use crate::workspace::domain::agent_runner::{
    AgentRunError, AgentRunEvent, AgentRunEventSink, AgentRunSpec, AgentRunner, RunFinalState,
    RunHandle,
};

/// The workspace `AgentRunner` backed by `ai_session`, wired into the app by
/// `workspace::build` and driven by the `run_agent` use case (#80).
pub struct AiSessionAgentRunner {
    app: AppHandle,
}

impl AiSessionAgentRunner {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait]
impl AgentRunner for AiSessionAgentRunner {
    async fn start(
        &self,
        spec: AgentRunSpec,
        sink: AgentRunEventSink,
    ) -> Result<RunHandle, AgentRunError> {
        if spec.prompt.trim().is_empty() {
            return Err(AgentRunError::EmptyPrompt);
        }

        let registry = self.app.state::<SessionRegistry>();
        let adapter =
            registry
                .adapters()
                .get(&spec.tool_id)
                .ok_or_else(|| AgentRunError::UnknownTool {
                    tool_id: spec.tool_id.clone(),
                })?;

        let spawn_spec = build_spawn_spec(adapter, &spec)?;

        let (session_sink, session_events) = mpsc::unbounded_channel();
        let spawned = process::spawn_with_sink(
            self.app.clone(),
            &registry,
            spec.tool_id.clone(),
            spawn_spec,
            Some(session_sink),
        )
        .await
        .map_err(|reason| AgentRunError::Failed { reason })?;

        let run_id = spawned.session_id.clone();
        let _ = sink.send(AgentRunEvent::Started {
            run_id: run_id.clone(),
        });
        spawn_forwarder(self.app.clone(), spec.tool_id, session_events, sink);

        Ok(RunHandle { run_id })
    }

    async fn stop(&self, handle: &RunHandle) -> Result<(), AgentRunError> {
        let registry = self.app.state::<SessionRegistry>();
        process::kill_session(&registry, &handle.run_id).map_err(|_| AgentRunError::RunNotFound {
            run_id: handle.run_id.clone(),
        })
    }
}

/// Translates an `AgentRunSpec` into the process runner's `SpawnRequest` →
/// `SpawnSpec`. Trusted-tools is always on: a headless run cannot answer
/// interactive prompts, and the workspace permission floor is the safety net.
fn build_spawn_spec(
    adapter: &dyn CliAdapter,
    spec: &AgentRunSpec,
) -> Result<crate::ai_session::adapter::SpawnSpec, AgentRunError> {
    let request = SpawnRequest {
        prompt: spec.prompt.clone(),
        working_directory: spec.working_directory.clone(),
        agent: None,
        resume_id: spec.resume_id.clone(),
        shell: None,
        environment: Vec::new(),
        trust_all_tools: true,
    };
    adapter
        .build_spec(&request, None)
        .map_err(|reason| AgentRunError::Failed { reason })
}

/// Consumes the session's in-process events on a background task and forwards
/// them as `AgentRunEvent`s. The conversation id is read from stdout lines by
/// the tool's own parser, resolved once up front so the loop holds no registry
/// borrow across awaits.
fn spawn_forwarder(
    app: AppHandle,
    tool_id: String,
    mut session_events: mpsc::UnboundedReceiver<AiSessionEvent>,
    sink: AgentRunEventSink,
) {
    tauri::async_runtime::spawn(async move {
        let read_conversation_id = move |line: &str| {
            let registry = app.state::<SessionRegistry>();
            registry
                .adapters()
                .get(&tool_id)
                .and_then(|adapter| adapter.conversation_id(line))
        };
        let mut conversation_id_seen = false;
        while let Some(event) = session_events.recv().await {
            if forward_event(
                event,
                &sink,
                &read_conversation_id,
                &mut conversation_id_seen,
            )
            .is_break()
            {
                break;
            }
        }
    });
}

/// Maps one `AiSessionEvent` to the agent-run sink and reports whether the run
/// ended. Kept free of the `AppHandle` and process plumbing — the conversation
/// id is read through the injected `read_conversation_id` — so the mapping is
/// unit-testable with scripted events and the real Kiro parser, no binary.
fn forward_event(
    event: AiSessionEvent,
    sink: &AgentRunEventSink,
    read_conversation_id: &impl Fn(&str) -> Option<String>,
    conversation_id_seen: &mut bool,
) -> std::ops::ControlFlow<()> {
    match event {
        AiSessionEvent::Line { stream, line } => {
            if stream == OutputStream::Stdout && !*conversation_id_seen {
                if let Some(id) = read_conversation_id(&line) {
                    *conversation_id_seen = true;
                    let _ = sink.send(AgentRunEvent::ConversationId { id });
                }
            }
            let _ = sink.send(AgentRunEvent::Line { text: line });
            std::ops::ControlFlow::Continue(())
        }
        AiSessionEvent::Ended { status, exit_code } => {
            let _ = sink.send(AgentRunEvent::Ended {
                final_state: final_state_of(status),
                exit_code,
            });
            std::ops::ControlFlow::Break(())
        }
    }
}

fn final_state_of(status: SessionStatus) -> RunFinalState {
    match status {
        SessionStatus::Completed => RunFinalState::Finished,
        SessionStatus::Killed => RunFinalState::Stopped,
        SessionStatus::Running | SessionStatus::Failed => RunFinalState::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_session::kiro::KiroAdapter;

    const CONVERSATION_LINE: &str = r#"{"type":"metadata","data":{"sessionId":"conv-123"}}"#;
    const PLAIN_LINE: &str = "working on it";
    const CONVERSATION_ID: &str = "conv-123";

    /// Feeds scripted session events through the mapping with the real Kiro
    /// parser and collects the resulting agent-run events.
    fn map_events(events: Vec<AiSessionEvent>) -> Vec<AgentRunEvent> {
        let kiro = KiroAdapter;
        let read_conversation_id = |line: &str| kiro.conversation_id(line);
        let (sink, mut received) = mpsc::unbounded_channel();
        let mut conversation_id_seen = false;
        for event in events {
            let _ = forward_event(
                event,
                &sink,
                &read_conversation_id,
                &mut conversation_id_seen,
            );
        }
        drop(sink);
        let mut collected = Vec::new();
        while let Ok(event) = received.try_recv() {
            collected.push(event);
        }
        collected
    }

    fn stdout_line(line: &str) -> AiSessionEvent {
        AiSessionEvent::Line {
            stream: OutputStream::Stdout,
            line: line.to_string(),
        }
    }

    #[test]
    fn a_stdout_line_is_forwarded_as_a_line_event() {
        let events = map_events(vec![stdout_line(PLAIN_LINE)]);

        assert_eq!(
            events,
            vec![AgentRunEvent::Line {
                text: PLAIN_LINE.to_string()
            }]
        );
    }

    #[test]
    fn the_conversation_id_is_surfaced_before_its_line_and_only_once() {
        let events = map_events(vec![
            stdout_line(CONVERSATION_LINE),
            stdout_line(CONVERSATION_LINE),
        ]);

        assert_eq!(
            events,
            vec![
                AgentRunEvent::ConversationId {
                    id: CONVERSATION_ID.to_string()
                },
                AgentRunEvent::Line {
                    text: CONVERSATION_LINE.to_string()
                },
                AgentRunEvent::Line {
                    text: CONVERSATION_LINE.to_string()
                },
            ]
        );
    }

    #[test]
    fn a_stderr_line_is_forwarded_without_reading_a_conversation_id() {
        let events = map_events(vec![AiSessionEvent::Line {
            stream: OutputStream::Stderr,
            line: CONVERSATION_LINE.to_string(),
        }]);

        assert_eq!(
            events,
            vec![AgentRunEvent::Line {
                text: CONVERSATION_LINE.to_string()
            }]
        );
    }

    #[test]
    fn a_completed_run_ends_as_finished() {
        let events = map_events(vec![AiSessionEvent::Ended {
            status: SessionStatus::Completed,
            exit_code: Some(0),
        }]);

        assert_eq!(
            events,
            vec![AgentRunEvent::Ended {
                final_state: RunFinalState::Finished,
                exit_code: Some(0),
            }]
        );
    }

    #[test]
    fn a_killed_run_ends_as_stopped() {
        let events = map_events(vec![AiSessionEvent::Ended {
            status: SessionStatus::Killed,
            exit_code: None,
        }]);

        assert_eq!(
            events,
            vec![AgentRunEvent::Ended {
                final_state: RunFinalState::Stopped,
                exit_code: None,
            }]
        );
    }

    #[test]
    fn a_failed_run_ends_as_failed() {
        let events = map_events(vec![AiSessionEvent::Ended {
            status: SessionStatus::Failed,
            exit_code: Some(1),
        }]);

        assert_eq!(
            events,
            vec![AgentRunEvent::Ended {
                final_state: RunFinalState::Failed,
                exit_code: Some(1),
            }]
        );
    }

    #[test]
    fn the_ended_event_stops_forwarding() {
        let kiro = KiroAdapter;
        let read_conversation_id = |line: &str| kiro.conversation_id(line);
        let (sink, _received) = mpsc::unbounded_channel();
        let mut conversation_id_seen = false;

        let flow = forward_event(
            AiSessionEvent::Ended {
                status: SessionStatus::Completed,
                exit_code: Some(0),
            },
            &sink,
            &read_conversation_id,
            &mut conversation_id_seen,
        );

        assert!(flow.is_break());
    }
}
