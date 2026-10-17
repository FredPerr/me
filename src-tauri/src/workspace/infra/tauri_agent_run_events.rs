//! The typed `agent-run` Tauri event. The run-driving use case emits one as a
//! run streams (started, a line, ended); the frontend subscribes to render the
//! live transcript (#81). Modelled on `TauriGitHostingEvents`: a serde-tagged
//! payload exported to `src/bindings.ts` through tauri-specta.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::workspace::domain::agent_runner::{AgentRunEvents, AgentRunUpdate, RunFinalState};

/// How a run ended, on the wire. Mirrors the domain `RunFinalState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RunFinalStateView {
    Finished,
    Failed,
    Stopped,
}

impl From<RunFinalState> for RunFinalStateView {
    fn from(state: RunFinalState) -> Self {
        match state {
            RunFinalState::Finished => Self::Finished,
            RunFinalState::Failed => Self::Failed,
            RunFinalState::Stopped => Self::Stopped,
        }
    }
}

/// What happened to the run, on the wire. One variant per thing the UI renders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AgentRunUpdateView {
    Started,
    #[serde(rename_all = "camelCase")]
    Line {
        text: String,
    },
    #[serde(rename_all = "camelCase")]
    Ended {
        final_state: RunFinalStateView,
        exit_code: Option<i32>,
    },
}

impl From<AgentRunUpdate> for AgentRunUpdateView {
    fn from(update: AgentRunUpdate) -> Self {
        match update {
            AgentRunUpdate::Started => Self::Started,
            AgentRunUpdate::Line { text } => Self::Line { text },
            AgentRunUpdate::Ended {
                final_state,
                exit_code,
            } => Self::Ended {
                final_state: final_state.into(),
                exit_code,
            },
        }
    }
}

/// Emitted as `agent-run` as a context's run streams, carrying the context and
/// run it concerns so the frontend can route it to the right transcript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunChanged {
    pub context_id: String,
    pub run_id: String,
    pub update: AgentRunUpdateView,
}

pub struct TauriAgentRunEvents {
    app: AppHandle,
}

impl TauriAgentRunEvents {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl AgentRunEvents for TauriAgentRunEvents {
    fn run_changed(&self, context_id: &str, run_id: &str, update: AgentRunUpdate) {
        let event = AgentRunChanged {
            context_id: context_id.to_string(),
            run_id: run_id.to_string(),
            update: update.into(),
        };
        if let Err(error) = event.emit(&self.app) {
            eprintln!("[workspace] agent-run for context '{context_id}' not emitted: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTEXT_ID: &str = "context-1";
    const RUN_ID: &str = "run-1";

    #[test]
    fn is_named_agent_run_changed() {
        // tauri-specta derives the event name from the struct, kebab-cased,
        // like `git-hosting-changed`. The frontend (#81) subscribes to this.
        assert_eq!(AgentRunChanged::NAME, "agent-run-changed");
    }

    #[test]
    fn serializes_a_started_update() {
        let event = AgentRunChanged {
            context_id: CONTEXT_ID.to_string(),
            run_id: RUN_ID.to_string(),
            update: AgentRunUpdateView::Started,
        };

        assert_eq!(
            serde_json::to_value(event).unwrap(),
            serde_json::json!({
                "contextId": CONTEXT_ID,
                "runId": RUN_ID,
                "update": { "kind": "started" },
            })
        );
    }

    #[test]
    fn serializes_a_line_update() {
        let update = AgentRunUpdateView::from(AgentRunUpdate::Line {
            text: "working".to_string(),
        });

        assert_eq!(
            serde_json::to_value(update).unwrap(),
            serde_json::json!({ "kind": "line", "text": "working" })
        );
    }

    #[test]
    fn serializes_an_ended_update_with_its_state_and_code() {
        let update = AgentRunUpdateView::from(AgentRunUpdate::Ended {
            final_state: RunFinalState::Finished,
            exit_code: Some(0),
        });

        assert_eq!(
            serde_json::to_value(update).unwrap(),
            serde_json::json!({ "kind": "ended", "finalState": "finished", "exitCode": 0 })
        );
    }
}
