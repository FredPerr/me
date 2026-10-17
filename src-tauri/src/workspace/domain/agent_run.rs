//! The transcript of one agent run: a prompt, the lines the agent produced,
//! and how the run ended. The two caps that keep transcripts bounded live here
//! as pure logic so they can be tested without any storage:
//!
//! - A single transcript holds at most `MAX_TRANSCRIPT_BYTES` of output. "Size"
//!   is the sum of each line's UTF-8 byte length (`str::len`); the newlines
//!   between lines are not counted. Appending beyond the cap drops the oldest
//!   lines until the rest fit and sets `trimmed`.
//! - A context keeps at most `MAX_RUNS_PER_CONTEXT` runs. That cap is a
//!   property of a context's run list, enforced by the repository on save; the
//!   free function `keep_recent_runs` applies it so both repository adapters
//!   share one rule.

use serde::{Deserialize, Serialize};

use super::identifiers::{AgentRunId, ContextId};

/// The largest transcript kept per run, 200 KB. Past this the oldest output
/// lines are dropped so the UI can still load a transcript in one piece.
pub const MAX_TRANSCRIPT_BYTES: usize = 200 * 1024;

/// The number of runs a context keeps. Starting an eleventh run drops the
/// oldest, so old runs don't pile up.
pub const MAX_RUNS_PER_CONTEXT: usize = 10;

/// How a run ended, or that it is still going. A run left `Running` in storage
/// is one whose app closed mid-run; the next start reconciles it (ticket #82).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentRunState {
    /// Still going. A crash leaves a run in this state to reconcile later.
    Running,
    /// The agent exited on its own.
    Finished,
    /// The agent exited reporting failure.
    Failed,
    /// The user stopped the run.
    Stopped,
    /// The run was cut short by something other than the user (e.g. the app
    /// closing), including a `Running` run found after a crash.
    Interrupted,
}

impl AgentRunState {
    /// Whether the run is still going and so needs reconciling after a crash.
    pub fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }
}

/// One run of an agent in a context: the prompt that started it, the output
/// lines it produced (oldest first), its state and, once ended, its exit code.
/// `trimmed` marks a transcript whose oldest lines were dropped to stay under
/// `MAX_TRANSCRIPT_BYTES`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub id: AgentRunId,
    pub context_id: ContextId,
    pub prompt: String,
    pub output_lines: Vec<String>,
    pub state: AgentRunState,
    pub exit_code: Option<i32>,
    pub trimmed: bool,
    /// When the run started, ISO 8601 as the caller recorded it, matching the
    /// other timestamp fields in storage. The repository does not read it.
    pub started_at: String,
}

impl AgentRun {
    /// A fresh, still-running transcript with no output yet.
    pub fn started(
        id: AgentRunId,
        context_id: ContextId,
        prompt: String,
        started_at: String,
    ) -> Self {
        let mut run = Self {
            id,
            context_id,
            prompt,
            output_lines: Vec::new(),
            state: AgentRunState::Running,
            exit_code: None,
            trimmed: false,
            started_at,
        };
        run.enforce_transcript_cap();
        run
    }

    /// Appends output lines, then drops the oldest lines until the transcript
    /// fits in `MAX_TRANSCRIPT_BYTES`, setting `trimmed` if any were dropped.
    pub fn append_lines(&mut self, lines: impl IntoIterator<Item = String>) {
        self.output_lines.extend(lines);
        self.enforce_transcript_cap();
    }

    /// Records how the run ended and its exit code, leaving the transcript be.
    pub fn end(&mut self, state: AgentRunState, exit_code: Option<i32>) {
        self.state = state;
        self.exit_code = exit_code;
    }

    /// The transcript size: the sum of the lines' UTF-8 byte lengths.
    pub fn transcript_bytes(&self) -> usize {
        self.output_lines.iter().map(String::len).sum()
    }

    /// Drops the oldest lines until the transcript fits, setting `trimmed` when
    /// it has to. A single line longer than the cap is kept (dropping it would
    /// empty the transcript and still not fit), and that too counts as trimmed.
    fn enforce_transcript_cap(&mut self) {
        let mut total = self.transcript_bytes();
        let mut dropped = 0;
        while total > MAX_TRANSCRIPT_BYTES && self.output_lines.len() > 1 {
            total -= self.output_lines[dropped].len();
            dropped += 1;
        }
        if dropped > 0 {
            self.output_lines.drain(0..dropped);
            self.trimmed = true;
        }
        if self.transcript_bytes() > MAX_TRANSCRIPT_BYTES {
            self.trimmed = true;
        }
    }
}

/// Keeps only the most recent `MAX_RUNS_PER_CONTEXT` runs of a context,
/// dropping the oldest. `runs` is oldest first, so the oldest are at the front.
pub fn keep_recent_runs(mut runs: Vec<AgentRun>) -> Vec<AgentRun> {
    if runs.len() > MAX_RUNS_PER_CONTEXT {
        let excess = runs.len() - MAX_RUNS_PER_CONTEXT;
        runs.drain(0..excess);
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTEXT: &str = "context-1";
    const PROMPT: &str = "Build the login page";
    const STARTED_AT: &str = "2024-01-15T10:00:00.000Z";
    const SHORT_LINE: &str = "a line of output";

    fn run_with_lines(lines: Vec<String>) -> AgentRun {
        let mut run = AgentRun::started(
            AgentRunId::generate(),
            ContextId::parse(CONTEXT).unwrap(),
            PROMPT.to_string(),
            STARTED_AT.to_string(),
        );
        run.append_lines(lines);
        run
    }

    fn line_of(bytes: usize) -> String {
        "x".repeat(bytes)
    }

    #[test]
    fn a_started_run_is_running_with_no_output() {
        let run = AgentRun::started(
            AgentRunId::generate(),
            ContextId::parse(CONTEXT).unwrap(),
            PROMPT.to_string(),
            STARTED_AT.to_string(),
        );

        assert_eq!(run.state, AgentRunState::Running);
        assert!(run.output_lines.is_empty());
        assert!(!run.trimmed);
        assert_eq!(run.exit_code, None);
    }

    #[test]
    fn append_keeps_lines_oldest_first() {
        let run = run_with_lines(vec!["first".to_string(), "second".to_string()]);

        assert_eq!(run.output_lines, vec!["first", "second"]);
        assert!(!run.trimmed);
    }

    #[test]
    fn transcript_size_sums_line_byte_lengths() {
        let run = run_with_lines(vec![SHORT_LINE.to_string(), SHORT_LINE.to_string()]);

        assert_eq!(run.transcript_bytes(), SHORT_LINE.len() * 2);
    }

    #[test]
    fn appending_past_the_cap_drops_the_oldest_lines_and_sets_trimmed() {
        let under_cap_line = line_of(MAX_TRANSCRIPT_BYTES / 2);
        let mut run = run_with_lines(vec!["oldest".to_string(), under_cap_line.clone()]);

        run.append_lines(vec![under_cap_line.clone()]);

        assert!(run.trimmed);
        assert_eq!(
            run.output_lines,
            vec![under_cap_line.clone(), under_cap_line]
        );
        assert!(run.transcript_bytes() <= MAX_TRANSCRIPT_BYTES);
    }

    #[test]
    fn a_transcript_within_the_cap_is_not_trimmed() {
        let run = run_with_lines(vec![line_of(MAX_TRANSCRIPT_BYTES)]);

        assert!(!run.trimmed);
        assert_eq!(run.transcript_bytes(), MAX_TRANSCRIPT_BYTES);
    }

    #[test]
    fn a_single_line_larger_than_the_cap_is_kept_and_marked_trimmed() {
        let run = run_with_lines(vec![line_of(MAX_TRANSCRIPT_BYTES + 1)]);

        assert_eq!(run.output_lines.len(), 1);
        assert!(run.trimmed);
    }

    #[test]
    fn ending_a_run_records_its_state_and_exit_code() {
        let mut run = run_with_lines(vec![]);

        run.end(AgentRunState::Finished, Some(0));

        assert_eq!(run.state, AgentRunState::Finished);
        assert_eq!(run.exit_code, Some(0));
    }

    #[test]
    fn keep_recent_runs_drops_the_oldest_beyond_the_limit() {
        let runs: Vec<AgentRun> = (0..MAX_RUNS_PER_CONTEXT + 1)
            .map(|index| run_with_lines(vec![index.to_string()]))
            .collect();
        let newest = runs.last().unwrap().clone();

        let kept = keep_recent_runs(runs);

        assert_eq!(kept.len(), MAX_RUNS_PER_CONTEXT);
        assert_eq!(kept.last().unwrap(), &newest);
        assert_eq!(kept.first().unwrap().output_lines, vec!["1"]);
    }

    #[test]
    fn keep_recent_runs_leaves_a_short_list_alone() {
        let runs = vec![run_with_lines(vec![]), run_with_lines(vec![])];

        let kept = keep_recent_runs(runs.clone());

        assert_eq!(kept, runs);
    }

    #[test]
    fn running_state_is_recognised() {
        assert!(AgentRunState::Running.is_running());
        assert!(!AgentRunState::Finished.is_running());
    }
}
