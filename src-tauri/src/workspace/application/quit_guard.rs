//! The quit-guard decision (#82): given the agent runs the current session has
//! in progress, decide whether closing the window may go ahead or must ask the
//! user first, and what that confirmation says.
//!
//! This is a pure function so it can be tested without a window or a dialog.
//! The Tauri wiring around it (preventing the close, showing the native dialog,
//! stopping the runs) is a thin shell over this decision and over
//! [`AgentRunService::in_progress_runs`] and
//! [`AgentRunService::stop_all_in_progress`].

use crate::workspace::domain::identifiers::ContextId;

/// One agent run the current session is driving: the context it runs in and
/// that context's name, so the confirmation can name what would be stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunInProgress {
    pub context_id: ContextId,
    pub context_name: String,
}

/// What to do when the window is asked to close while runs may be in progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseDecision {
    /// No run is in progress; let the close go ahead without asking.
    AllowClose,
    /// Runs are in progress; prevent the close and ask the user first with
    /// `message`, offering "Stop runs and quit" and "Cancel".
    Confirm { message: String },
}

/// Decides whether closing may proceed. With no runs in progress the close is
/// allowed; otherwise it must be confirmed with a message naming the contexts
/// whose runs would be stopped, in the order given.
pub fn decide_close(runs_in_progress: &[RunInProgress]) -> CloseDecision {
    if runs_in_progress.is_empty() {
        return CloseDecision::AllowClose;
    }
    CloseDecision::Confirm {
        message: confirmation_message(runs_in_progress),
    }
}

/// "An agent run is in progress: login-page" for one run, or
/// "2 agent runs are in progress: login-page, billing-export" for several.
fn confirmation_message(runs_in_progress: &[RunInProgress]) -> String {
    let names = runs_in_progress
        .iter()
        .map(|run| run.context_name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if runs_in_progress.len() == 1 {
        format!("An agent run is in progress: {names}")
    } else {
        let count = runs_in_progress.len();
        format!("{count} agent runs are in progress: {names}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOGIN_PAGE: &str = "login-page";
    const BILLING_EXPORT: &str = "billing-export";

    fn run_in(context_name: &str) -> RunInProgress {
        RunInProgress {
            context_id: ContextId::generate(),
            context_name: context_name.to_string(),
        }
    }

    #[test]
    fn allows_the_close_when_no_run_is_in_progress() {
        let decision = decide_close(&[]);

        assert_eq!(decision, CloseDecision::AllowClose);
    }

    #[test]
    fn asks_with_a_singular_message_for_one_run() {
        let decision = decide_close(&[run_in(LOGIN_PAGE)]);

        assert_eq!(
            decision,
            CloseDecision::Confirm {
                message: "An agent run is in progress: login-page".to_string(),
            }
        );
    }

    #[test]
    fn asks_with_the_counted_list_for_several_runs_in_order() {
        let decision = decide_close(&[run_in(LOGIN_PAGE), run_in(BILLING_EXPORT)]);

        assert_eq!(
            decision,
            CloseDecision::Confirm {
                message: "2 agent runs are in progress: login-page, billing-export".to_string(),
            }
        );
    }
}
