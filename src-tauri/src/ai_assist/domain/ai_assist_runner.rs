//! The `AiAssistRunner` port: run an AI tool to completion, read-only, in a
//! folder with a prompt, and return the reconstructed answer — or a typed
//! error. A crate-level composition adapter implements it over `ai_session`;
//! `ai_assist` never names that module.
//!
//! An AI assist is fire-and-forget-then-await: the caller starts it, shows
//! "Generating…", and waits for the single result. There is no streaming and
//! nothing to observe mid-run, so unlike spec 5's `AgentRunner` the port
//! exposes one awaitable `run` that resolves with the finished answer.
//!
//! Cancel is modelled as a [`CancelSignal`] the caller passes into `run`: a
//! thin wrapper over a `oneshot` receiver that resolves when its paired
//! [`CancelTrigger`] fires (or is dropped). The run races the signal against
//! the tool finishing and the timeout. The command layer (#88/#89) holds the
//! trigger keyed by an assist id so a UI cancel button can fire it. This keeps
//! the port a single call with no separate handle to track, and both sides are
//! trivially testable with an in-memory pair.

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::oneshot;

/// The three-minute ceiling after which a run that has not finished is killed
/// and reported as [`AiAssistRunError::TimedOut`]. A run can never hang.
pub const AI_ASSIST_TIMEOUT: Duration = Duration::from_secs(180);

/// What to run: the tool, the folder it runs in, and the prompt. An AI assist
/// is always read-only, so there is deliberately no trust field — the runner
/// forces read-only for every run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiAssistRunSpec {
    /// The AI tool to run, by its adapter id (e.g. `"kiro"`).
    pub tool_id: String,
    /// Directory the tool runs in (a temp folder for context suggestions, or a
    /// context worktree for pull-request suggestions).
    pub working_directory: String,
    /// The instruction to run headlessly.
    pub prompt: String,
}

/// Why an AI assist did not return an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiAssistRunError {
    /// The caller cancelled the run through its [`CancelSignal`].
    Cancelled,
    /// The run exceeded [`AI_ASSIST_TIMEOUT`] and was killed.
    TimedOut,
    /// The run could not start or ended in a non-success terminal state that
    /// was not a cancel or timeout.
    Failed { reason: String },
}

impl fmt::Display for AiAssistRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => write!(formatter, "The AI assist was cancelled"),
            Self::TimedOut => write!(
                formatter,
                "The AI assist ran longer than {} seconds and was stopped",
                AI_ASSIST_TIMEOUT.as_secs()
            ),
            Self::Failed { reason } => write!(formatter, "The AI assist failed: {reason}"),
        }
    }
}

impl std::error::Error for AiAssistRunError {}

/// The caller's half of a cancel: firing it (or dropping it) resolves the
/// paired [`CancelSignal`]. Build a pair with [`cancel_channel`]. The command
/// layer keeps the trigger alive, keyed by an assist id, until the run ends or
/// the user cancels.
#[derive(Debug)]
pub struct CancelTrigger {
    sender: oneshot::Sender<()>,
}

impl CancelTrigger {
    /// Requests cancellation. A no-op if the run has already ended (the signal
    /// side has been dropped).
    pub fn cancel(self) {
        let _ = self.sender.send(());
    }
}

/// The run's half of a cancel: an awaitable that completes once the paired
/// [`CancelTrigger`] fires or is dropped. The runner selects on this alongside
/// the tool finishing and the timeout.
#[derive(Debug)]
pub struct CancelSignal {
    receiver: oneshot::Receiver<()>,
}

impl CancelSignal {
    /// Resolves when cancellation is requested (or the trigger is dropped).
    pub async fn cancelled(self) {
        // A send resolves to Ok; a dropped trigger resolves to Err. Either way
        // the intent is "stop", so both outcomes complete the await.
        let _ = self.receiver.await;
    }
}

/// Creates a linked [`CancelTrigger`]/[`CancelSignal`] pair. The trigger is
/// handed to whoever may cancel (the command layer); the signal is passed into
/// [`AiAssistRunner::run`].
pub fn cancel_channel() -> (CancelTrigger, CancelSignal) {
    let (sender, receiver) = oneshot::channel();
    (CancelTrigger { sender }, CancelSignal { receiver })
}

/// Runs an AI tool to completion, read-only, and returns its answer. Implemented
/// by a composition adapter over the process runner; callers depend only on
/// this trait.
#[async_trait]
pub trait AiAssistRunner: Send + Sync {
    /// Runs `spec` read-only to completion and returns the reconstructed answer
    /// text, racing the run against the `cancel` signal and the three-minute
    /// timeout. On cancel or timeout the underlying run is killed.
    async fn run(
        &self,
        spec: AiAssistRunSpec,
        cancel: CancelSignal,
    ) -> Result<String, AiAssistRunError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_signal_resolves_when_the_trigger_fires() {
        let (trigger, signal) = cancel_channel();

        trigger.cancel();

        // Completing the await is the whole contract; it must not hang.
        signal.cancelled().await;
    }

    #[tokio::test]
    async fn the_signal_resolves_when_the_trigger_is_dropped() {
        let (trigger, signal) = cancel_channel();

        drop(trigger);

        signal.cancelled().await;
    }
}
