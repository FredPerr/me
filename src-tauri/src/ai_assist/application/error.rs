//! The error a suggestion use case returns. It joins the typed run failures
//! (cancelled, timed out, failed) and the typed parse failures (the four bulk
//! parse kinds) into one enum the command layer maps to the frontend
//! `AiAssistError`.

use crate::ai_assist::domain::ai_assist_runner::AiAssistRunError;
use crate::ai_assist::domain::bulk_parser::BulkParseError;
use crate::ai_assist::domain::ports::{PullRequestContextError, RepositoryDiffUnavailable};

/// Why a suggestion did not produce drafts. Each variant is a distinct thing
/// the user is told about; the command layer turns it into the matching
/// `AiAssistError` kind and message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestContextsError {
    /// The instruction was blank, so there was nothing to expand.
    EmptyInstruction,
    /// The user cancelled the run.
    Cancelled,
    /// The run exceeded the three-minute limit and was stopped.
    TimedOut,
    /// The run could not start or ended without an answer.
    Failed { reason: String },
    /// The answer held no `[...]` array (frontend `no-json-found`).
    NoListFound,
    /// An array was found but is malformed JSON (frontend `invalid-json`).
    Malformed,
    /// The parsed JSON is not an array (frontend `not-an-array`).
    NotAList,
    /// No row survived parsing, or no parsed draft's branch could be named
    /// under the project's policy (frontend `no-valid-rows`).
    NoValidEntries,
}

impl From<AiAssistRunError> for SuggestContextsError {
    fn from(error: AiAssistRunError) -> Self {
        match error {
            AiAssistRunError::Cancelled => Self::Cancelled,
            AiAssistRunError::TimedOut => Self::TimedOut,
            AiAssistRunError::Failed { reason } => Self::Failed { reason },
        }
    }
}

impl From<BulkParseError> for SuggestContextsError {
    fn from(error: BulkParseError) -> Self {
        match error {
            BulkParseError::NoListFound => Self::NoListFound,
            BulkParseError::Malformed => Self::Malformed,
            BulkParseError::NotAList => Self::NotAList,
            BulkParseError::NoValidEntries => Self::NoValidEntries,
        }
    }
}

/// Why a pull-request suggestion did not produce a title and a description. It
/// shares the run failures (cancelled, timed out, failed) with
/// [`SuggestContextsError`] and adds the two input problems specific to this
/// use case: no changes to describe, and an answer with no text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestPullRequestError {
    /// The context, repository or project could not be resolved.
    NotFound { reason: String },
    /// There is no base branch to compare the changes against.
    NoBaseBranch,
    /// The branch has no changes against the base, so there is nothing to write.
    EmptyDiff,
    /// The run produced no usable text.
    EmptyAnswer,
    /// The user cancelled the run.
    Cancelled,
    /// The run exceeded the three-minute limit and was stopped.
    TimedOut,
    /// The run could not start or ended without an answer.
    Failed { reason: String },
}

impl From<AiAssistRunError> for SuggestPullRequestError {
    fn from(error: AiAssistRunError) -> Self {
        match error {
            AiAssistRunError::Cancelled => Self::Cancelled,
            AiAssistRunError::TimedOut => Self::TimedOut,
            AiAssistRunError::Failed { reason } => Self::Failed { reason },
        }
    }
}

impl From<PullRequestContextError> for SuggestPullRequestError {
    fn from(error: PullRequestContextError) -> Self {
        match error {
            PullRequestContextError::NotFound { reason } => Self::NotFound { reason },
            PullRequestContextError::NoBaseBranch => Self::NoBaseBranch,
        }
    }
}

impl From<RepositoryDiffUnavailable> for SuggestPullRequestError {
    fn from(error: RepositoryDiffUnavailable) -> Self {
        Self::Failed {
            reason: error.reason,
        }
    }
}
