//! Typed Tauri commands of the AI-assist context and their wire types.
//!
//! `ai_assist_suggest_contexts` runs the suggestion and returns the named
//! drafts or a typed [`AiAssistCommandError`]. `ai_assist_cancel_contexts`
//! fires the cancel for whatever suggestion is in flight — the modal runs one
//! at a time, so no id is needed.

use serde::Serialize;
use tauri::State;

use super::application::error::{SuggestContextsError, SuggestPullRequestError};
use super::domain::context_draft::ContextDraft;
use super::domain::suggested_pull_request::SuggestedPullRequest;
use super::AiAssist;

/// A ready-to-create context draft on the wire. `branchName` is valid under the
/// project's naming policy; `prompt` is the optional Kiro prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContextDraftView {
    pub context_name: String,
    pub branch_name: String,
    pub prompt: Option<String>,
}

impl From<ContextDraft> for ContextDraftView {
    fn from(draft: ContextDraft) -> Self {
        Self {
            context_name: draft.context_name,
            branch_name: draft.branch_name,
            prompt: draft.prompt,
        }
    }
}

/// A written pull request on the wire: both halves the modal needs. The modal
/// keeps a title the user already typed and always fills the description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedPullRequestView {
    pub title: String,
    pub description: String,
}

impl From<SuggestedPullRequest> for SuggestedPullRequestView {
    fn from(suggestion: SuggestedPullRequest) -> Self {
        Self {
            title: suggestion.title,
            description: suggestion.description,
        }
    }
}

/// The kind of an AI-assist failure, as the frontend sees it. The four parse
/// kinds carry the same meaning (and, via i18n, the same message) as the
/// frontend bulk parser's `no-json-found`/`invalid-json`/`not-an-array`/
/// `no-valid-rows`; the run kinds are cancel, timeout and a catch-all failure;
/// `invalidInput` is a blank instruction. The pull-request suggestion adds
/// `emptyDiff` (no changes to describe) and `emptyAnswer` (the run produced no
/// text).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum AiAssistErrorKind {
    InvalidInput,
    NoListFound,
    Malformed,
    NotAList,
    NoValidEntries,
    EmptyDiff,
    EmptyAnswer,
    Cancelled,
    TimedOut,
    Failed,
}

/// A failed AI-assist command on the wire. `message` is a readable English
/// sentence; it may carry the tool's failure reason for the `failed` kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AiAssistCommandError {
    pub kind: AiAssistErrorKind,
    pub message: String,
}

impl From<SuggestContextsError> for AiAssistCommandError {
    fn from(error: SuggestContextsError) -> Self {
        let (kind, message): (AiAssistErrorKind, String) = match error {
            SuggestContextsError::EmptyInstruction => (
                AiAssistErrorKind::InvalidInput,
                "Enter an instruction to expand into contexts.".to_string(),
            ),
            SuggestContextsError::NoListFound => (
                AiAssistErrorKind::NoListFound,
                "Kiro did not return a usable list. Try rephrasing the instruction.".to_string(),
            ),
            SuggestContextsError::Malformed => (
                AiAssistErrorKind::Malformed,
                "Kiro returned malformed output. Try regenerating.".to_string(),
            ),
            SuggestContextsError::NotAList => (
                AiAssistErrorKind::NotAList,
                "Kiro did not return a list of contexts. Try regenerating.".to_string(),
            ),
            SuggestContextsError::NoValidEntries => (
                AiAssistErrorKind::NoValidEntries,
                "No valid contexts could be generated from that instruction.".to_string(),
            ),
            SuggestContextsError::Cancelled => (
                AiAssistErrorKind::Cancelled,
                "The suggestion was cancelled.".to_string(),
            ),
            SuggestContextsError::TimedOut => (
                AiAssistErrorKind::TimedOut,
                "The suggestion ran too long and was stopped. Try a simpler instruction."
                    .to_string(),
            ),
            SuggestContextsError::Failed { reason } => (
                AiAssistErrorKind::Failed,
                format!("Kiro run failed: {reason}"),
            ),
        };
        Self { kind, message }
    }
}

impl From<SuggestPullRequestError> for AiAssistCommandError {
    fn from(error: SuggestPullRequestError) -> Self {
        let (kind, message): (AiAssistErrorKind, String) = match error {
            SuggestPullRequestError::NotFound { reason } => (
                AiAssistErrorKind::Failed,
                format!("The pull request could not be prepared: {reason}"),
            ),
            SuggestPullRequestError::NoBaseBranch => (
                AiAssistErrorKind::Failed,
                "This repository has no base branch to compare the changes against.".to_string(),
            ),
            SuggestPullRequestError::EmptyDiff => (
                AiAssistErrorKind::EmptyDiff,
                "No changes found against the base branch.".to_string(),
            ),
            SuggestPullRequestError::EmptyAnswer => (
                AiAssistErrorKind::EmptyAnswer,
                "Kiro returned an empty description. Try regenerating.".to_string(),
            ),
            SuggestPullRequestError::Cancelled => (
                AiAssistErrorKind::Cancelled,
                "The suggestion was cancelled.".to_string(),
            ),
            SuggestPullRequestError::TimedOut => (
                AiAssistErrorKind::TimedOut,
                "The suggestion ran too long and was stopped. Try again.".to_string(),
            ),
            SuggestPullRequestError::Failed { reason } => (
                AiAssistErrorKind::Failed,
                format!("Kiro run failed: {reason}"),
            ),
        };
        Self { kind, message }
    }
}

/// Expands one instruction into named context drafts, read-only in a fresh
/// temp folder. Returns the drafts or a typed error.
#[tauri::command]
#[specta::specta]
pub async fn ai_assist_suggest_contexts(
    ai_assist: State<'_, AiAssist>,
    project_tag: String,
    instruction: String,
) -> Result<Vec<ContextDraftView>, AiAssistCommandError> {
    let drafts = ai_assist
        .service
        .suggest_contexts(&project_tag, &instruction)
        .await?;
    Ok(drafts.into_iter().map(ContextDraftView::from).collect())
}

/// Cancels the context suggestion in flight, if any. A no-op when none is
/// running, so the modal can call it freely on close.
#[tauri::command]
#[specta::specta]
pub async fn ai_assist_cancel_contexts(ai_assist: State<'_, AiAssist>) -> Result<(), ()> {
    ai_assist.service.cancel();
    Ok(())
}

/// Writes a pull request title and description for one repository tab,
/// read-only in the context's worktree. Returns both halves or a typed error.
#[tauri::command]
#[specta::specta]
pub async fn ai_assist_suggest_pull_request(
    ai_assist: State<'_, AiAssist>,
    project_tag: String,
    context_id: String,
    repository_id: String,
) -> Result<SuggestedPullRequestView, AiAssistCommandError> {
    let suggestion = ai_assist
        .service
        .suggest_pull_request(&project_tag, &context_id, &repository_id)
        .await?;
    Ok(SuggestedPullRequestView::from(suggestion))
}

/// Cancels the pull-request suggestion in flight, if any. A no-op when none is
/// running, so the modal can call it freely on close.
#[tauri::command]
#[specta::specta]
pub async fn ai_assist_cancel_pull_request(ai_assist: State<'_, AiAssist>) -> Result<(), ()> {
    ai_assist.service.cancel_pull_request();
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_draft_is_sent_in_camel_case() {
        let view = ContextDraftView::from(ContextDraft {
            context_name: "Add login".to_string(),
            branch_name: "feature/add-login".to_string(),
            prompt: Some("do login".to_string()),
        });

        assert_eq!(
            serde_json::to_value(view).unwrap(),
            json!({
                "contextName": "Add login",
                "branchName": "feature/add-login",
                "prompt": "do login",
            })
        );
    }

    #[test]
    fn each_error_kind_maps_to_its_wire_kind_and_message() {
        let cases = [
            (SuggestContextsError::EmptyInstruction, "invalidInput"),
            (SuggestContextsError::NoListFound, "noListFound"),
            (SuggestContextsError::Malformed, "malformed"),
            (SuggestContextsError::NotAList, "notAList"),
            (SuggestContextsError::NoValidEntries, "noValidEntries"),
            (SuggestContextsError::Cancelled, "cancelled"),
            (SuggestContextsError::TimedOut, "timedOut"),
        ];
        for (error, expected_kind) in cases {
            let wire = serde_json::to_value(AiAssistCommandError::from(error)).unwrap();

            assert_eq!(wire["kind"], json!(expected_kind));
            assert!(wire["message"].as_str().is_some_and(|m| !m.is_empty()));
        }
    }

    #[test]
    fn a_failed_run_sends_the_failed_kind_with_the_reason() {
        let error = AiAssistCommandError::from(SuggestContextsError::Failed {
            reason: "no binary".to_string(),
        });

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({ "kind": "failed", "message": "Kiro run failed: no binary" })
        );
    }

    #[test]
    fn a_suggested_pull_request_is_sent_in_camel_case() {
        let view = SuggestedPullRequestView::from(SuggestedPullRequest {
            title: "Add login".to_string(),
            description: "- Adds a login form".to_string(),
        });

        assert_eq!(
            serde_json::to_value(view).unwrap(),
            json!({ "title": "Add login", "description": "- Adds a login form" })
        );
    }

    #[test]
    fn the_pull_request_empty_kinds_map_to_their_wire_kinds() {
        let cases = [
            (SuggestPullRequestError::EmptyDiff, "emptyDiff"),
            (SuggestPullRequestError::EmptyAnswer, "emptyAnswer"),
            (SuggestPullRequestError::Cancelled, "cancelled"),
            (SuggestPullRequestError::TimedOut, "timedOut"),
            (SuggestPullRequestError::NoBaseBranch, "failed"),
        ];
        for (error, expected_kind) in cases {
            let wire = serde_json::to_value(AiAssistCommandError::from(error)).unwrap();

            assert_eq!(wire["kind"], json!(expected_kind));
            assert!(wire["message"].as_str().is_some_and(|m| !m.is_empty()));
        }
    }
}
