//! `ai_assist`'s `AiAssistRunner`, run by `ai_session`. The only place the two
//! modules meet: `ai_assist` depends on the port, `ai_session` on the process
//! runner, and this adapter bridges them.
//!
//! It resolves the tool's adapter, builds a read-only spawn spec, spawns the
//! child with an in-process sink, and on a background task buffers every stdout
//! line until the run reaches a terminal state. The `run` future then races
//! three outcomes with `tokio::select!`:
//!   - the run ends → reconstruct the answer from the buffer and classify the
//!     terminal status ([`reconstruct_outcome`]);
//!   - the three-minute timeout elapses → kill the session, return `TimedOut`;
//!   - the cancel signal fires → kill the session, return `Cancelled`.
//!
//! Testing the live spawn/timeout/cancel wiring against a real process is not
//! possible, so the mapping that matters — buffered output plus a terminal
//! status to a typed [`Result`] — is factored into the pure
//! [`reconstruct_outcome`] seam and unit-tested with scripted events and no
//! binary. The spawn/select shell around it stays thin.
//!
//! Nothing constructs this adapter yet: no command consumes it until #88/#89
//! add the suggestion use cases, so the whole module is dead-code until then,
//! matching the repo's pattern for not-yet-wired composition pieces. Its own
//! tests exercise the collect-and-reconstruct seam.
#![allow(dead_code)] // used by suggest_contexts/suggest_pull_request in #88/#89

use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use crate::ai_assist::domain::agent_text::extract_agent_text;
use crate::ai_assist::domain::ai_assist_runner::{
    AiAssistRunError, AiAssistRunSpec, AiAssistRunner, CancelSignal, AI_ASSIST_TIMEOUT,
};
use crate::ai_session::adapter::{CliAdapter, SpawnRequest, SpawnSpec, ToolTrust};
use crate::ai_session::process::{self, AiSessionEvent};
use crate::ai_session::registry::{SessionRegistry, SessionStatus};
use crate::ai_session::OutputStream;
use async_trait::async_trait;

/// Not yet constructed anywhere: no command consumes it until #88/#89 add the
/// suggestion use cases (see the module-level note).
pub struct AiSessionAiAssistRunner {
    app: AppHandle,
}

impl AiSessionAiAssistRunner {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait]
impl AiAssistRunner for AiSessionAiAssistRunner {
    async fn run(
        &self,
        spec: AiAssistRunSpec,
        cancel: CancelSignal,
    ) -> Result<String, AiAssistRunError> {
        let registry = self.app.state::<SessionRegistry>();
        let adapter =
            registry
                .adapters()
                .get(&spec.tool_id)
                .ok_or_else(|| AiAssistRunError::Failed {
                    reason: format!("Unknown AI tool '{}'", spec.tool_id),
                })?;

        let spawn_spec = build_read_only_spawn_spec(adapter, &spec)?;

        let (session_sink, session_events) = mpsc::unbounded_channel();
        let spawned = process::spawn_with_sink(
            self.app.clone(),
            &registry,
            spec.tool_id.clone(),
            spawn_spec,
            Some(session_sink),
        )
        .await
        .map_err(|reason| AiAssistRunError::Failed { reason })?;

        let session_id = spawned.session_id;
        // Buffer the run's output and terminal status on a background task; the
        // select below awaits its completion against the timeout and cancel.
        let collected = tokio::spawn(collect_run(session_events));

        let kill = || process::kill_session(&registry, &session_id);

        tokio::select! {
            run_result = collected => {
                // The collector task finishing yields the buffered lines and the
                // terminal status, which the pure seam turns into the outcome.
                let CollectedRun { lines, final_status } = run_result
                    .map_err(|error| AiAssistRunError::Failed {
                        reason: format!("The AI assist task failed to complete: {error}"),
                    })?;
                reconstruct_outcome(&lines, final_status)
            }
            _ = tokio::time::sleep(AI_ASSIST_TIMEOUT) => {
                let _ = kill();
                Err(AiAssistRunError::TimedOut)
            }
            _ = cancel.cancelled() => {
                let _ = kill();
                Err(AiAssistRunError::Cancelled)
            }
        }
    }
}

/// Translates an [`AiAssistRunSpec`] into a read-only [`SpawnSpec`]. An AI
/// assist always runs read-only, so trust is forced to [`ToolTrust::ReadOnly`]
/// here; the spec carries no trust field to override it.
fn build_read_only_spawn_spec(
    adapter: &dyn CliAdapter,
    spec: &AiAssistRunSpec,
) -> Result<SpawnSpec, AiAssistRunError> {
    let request = SpawnRequest {
        prompt: spec.prompt.clone(),
        working_directory: spec.working_directory.clone(),
        agent: None,
        resume_id: None,
        shell: None,
        environment: Vec::new(),
        tool_trust: ToolTrust::ReadOnly,
    };
    adapter
        .build_spec(&request, None)
        .map_err(|reason| AiAssistRunError::Failed { reason })
}

/// The buffered result of a run: the stdout lines, in order, and the terminal
/// status the process reached.
struct CollectedRun {
    lines: Vec<String>,
    final_status: SessionStatus,
}

/// Consumes the session's in-process events until the run ends, buffering every
/// stdout line (stderr is noise for a read-only assist) and keeping the
/// terminal status. If the channel closes before an `Ended` event (the spawner
/// dropped), the run is treated as failed.
async fn collect_run(mut session_events: mpsc::UnboundedReceiver<AiSessionEvent>) -> CollectedRun {
    let mut lines = Vec::new();
    while let Some(event) = session_events.recv().await {
        match event {
            AiSessionEvent::Line {
                stream: OutputStream::Stdout,
                line,
            } => lines.push(line),
            AiSessionEvent::Line { .. } => {}
            AiSessionEvent::Ended { status, .. } => {
                return CollectedRun {
                    lines,
                    final_status: status,
                };
            }
        }
    }
    CollectedRun {
        lines,
        final_status: SessionStatus::Failed,
    }
}

/// The pure mapping seam: buffered output plus the terminal status to the typed
/// outcome. A completed run reconstructs the agent's answer from the lines; any
/// other terminal status is a failure here (cancel and timeout never reach this
/// path — the select returns them before the run is read).
fn reconstruct_outcome(
    lines: &[String],
    final_status: SessionStatus,
) -> Result<String, AiAssistRunError> {
    match final_status {
        SessionStatus::Completed => Ok(extract_agent_text(lines)),
        SessionStatus::Killed => Err(AiAssistRunError::Failed {
            reason: "The AI tool was terminated before it finished".to_string(),
        }),
        SessionStatus::Failed | SessionStatus::Running => Err(AiAssistRunError::Failed {
            reason: "The AI tool exited without completing".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MESSAGE_CHUNK: &str = r#"{"type":"sessionUpdate","data":{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Hi"}}}}"#;

    fn stdout(line: &str) -> AiSessionEvent {
        AiSessionEvent::Line {
            stream: OutputStream::Stdout,
            line: line.to_string(),
        }
    }

    fn ended(status: SessionStatus) -> AiSessionEvent {
        AiSessionEvent::Ended {
            status,
            exit_code: None,
        }
    }

    /// Feeds scripted session events through the real collector task and the
    /// pure seam, so the whole buffer→reconstruct mapping is exercised without a
    /// process. A fake process source: a channel scripted with events.
    async fn run_outcome(events: Vec<AiSessionEvent>) -> Result<String, AiAssistRunError> {
        let (sink, receiver) = mpsc::unbounded_channel();
        for event in events {
            sink.send(event).expect("the receiver is still open");
        }
        drop(sink);
        let CollectedRun {
            lines,
            final_status,
        } = collect_run(receiver).await;
        reconstruct_outcome(&lines, final_status)
    }

    #[tokio::test]
    async fn a_completed_run_returns_the_reconstructed_answer() {
        let outcome =
            run_outcome(vec![stdout(MESSAGE_CHUNK), ended(SessionStatus::Completed)]).await;

        assert_eq!(outcome, Ok("Hi".to_string()));
    }

    #[tokio::test]
    async fn stderr_lines_are_not_part_of_the_answer() {
        let outcome = run_outcome(vec![
            AiSessionEvent::Line {
                stream: OutputStream::Stderr,
                line: "a warning".to_string(),
            },
            stdout(MESSAGE_CHUNK),
            ended(SessionStatus::Completed),
        ])
        .await;

        assert_eq!(outcome, Ok("Hi".to_string()));
    }

    #[tokio::test]
    async fn a_plain_text_completed_run_joins_its_lines() {
        let outcome = run_outcome(vec![
            stdout("line one"),
            stdout("line two"),
            ended(SessionStatus::Completed),
        ])
        .await;

        assert_eq!(outcome, Ok("line one\nline two".to_string()));
    }

    #[tokio::test]
    async fn a_failed_terminal_status_is_a_failure() {
        let outcome = run_outcome(vec![stdout(MESSAGE_CHUNK), ended(SessionStatus::Failed)]).await;

        assert!(matches!(outcome, Err(AiAssistRunError::Failed { .. })));
    }

    #[tokio::test]
    async fn a_killed_terminal_status_is_a_failure() {
        let outcome = run_outcome(vec![ended(SessionStatus::Killed)]).await;

        assert!(matches!(outcome, Err(AiAssistRunError::Failed { .. })));
    }

    #[tokio::test]
    async fn a_closed_channel_without_an_end_event_is_a_failure() {
        // The spawner dropped before announcing a terminal state.
        let outcome = run_outcome(vec![stdout(MESSAGE_CHUNK)]).await;

        assert!(matches!(outcome, Err(AiAssistRunError::Failed { .. })));
    }
}
