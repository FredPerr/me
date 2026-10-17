//! `suggest_contexts`: turn one instruction into a list of ready-to-create
//! context drafts, read-only, in a throwaway folder.
//!
//! The use case holds the ports it reads through and orchestrates them:
//!   1. read the default AI tool's adapter id,
//!   2. build the bulk prompt from the instruction,
//!   3. create a fresh temp folder (so the run can touch no worktree) and run
//!      the tool read-only in it, racing cancel and the three-minute timeout,
//!   4. parse the reconstructed answer into raw drafts,
//!   5. name each draft's branch under the project's policy, dropping the ones
//!      that cannot be named; if none survive, this is `NoValidEntries`.
//!
//! Cancel is held in the service, not passed per call: the modal runs one
//! suggestion at a time, so the service keeps a single in-flight
//! [`CancelTrigger`]. `suggest_contexts` installs a fresh trigger before the
//! run and clears it after; `cancel` fires whatever is installed. This keeps
//! the command surface to two calls with no id to track.

use std::sync::{Arc, Mutex};

use crate::ai_assist::domain::ai_assist_runner::{
    cancel_channel, AiAssistRunSpec, AiAssistRunner, CancelTrigger,
};
use crate::ai_assist::domain::bulk_parser::parse_context_drafts;
use crate::ai_assist::domain::bulk_prompt::build_bulk_context_prompt;
use crate::ai_assist::domain::context_draft::{ContextDraft, RawContextDraft};
use crate::ai_assist::domain::ports::{DefaultAiTool, ProjectBranchNaming, TempWorkspace};

use super::error::SuggestContextsError;

/// Orchestrates the context-suggestion use case over its ports, and holds the
/// single in-flight cancel trigger so a running suggestion can be cancelled.
pub struct AiAssistService {
    runner: Arc<dyn AiAssistRunner>,
    default_tool: Arc<dyn DefaultAiTool>,
    branch_naming: Arc<dyn ProjectBranchNaming>,
    temp_workspace: Arc<dyn TempWorkspace>,
    /// The trigger for the suggestion in flight, if any. One at a time: a new
    /// run replaces it, and it is cleared when the run ends.
    in_flight_cancel: Mutex<Option<CancelTrigger>>,
}

impl AiAssistService {
    pub fn new(
        runner: Arc<dyn AiAssistRunner>,
        default_tool: Arc<dyn DefaultAiTool>,
        branch_naming: Arc<dyn ProjectBranchNaming>,
        temp_workspace: Arc<dyn TempWorkspace>,
    ) -> Self {
        Self {
            runner,
            default_tool,
            branch_naming,
            temp_workspace,
            in_flight_cancel: Mutex::new(None),
        }
    }

    /// Suggests ready-to-create context drafts for `instruction` under
    /// `project_tag`. See the module note for the pipeline. Returns the named
    /// drafts, or a typed error.
    pub async fn suggest_contexts(
        &self,
        project_tag: &str,
        instruction: &str,
    ) -> Result<Vec<ContextDraft>, SuggestContextsError> {
        let instruction = instruction.trim();
        if instruction.is_empty() {
            return Err(SuggestContextsError::EmptyInstruction);
        }

        let tool_id = self.default_tool.tool_id();
        let prompt = build_bulk_context_prompt(instruction);

        let answer = self.run_in_temp_folder(tool_id, prompt).await?;

        let raw_drafts = parse_context_drafts(&answer)?;
        let named = self.name_branches(project_tag, &raw_drafts);
        if named.is_empty() {
            // Parity with the frontend: zero named drafts is no-valid-rows.
            return Err(SuggestContextsError::NoValidEntries);
        }
        Ok(named)
    }

    /// Cancels the suggestion in flight, if any. A no-op when none is running.
    pub fn cancel(&self) {
        if let Some(trigger) = self.take_in_flight_cancel() {
            trigger.cancel();
        }
    }

    /// Creates a fresh temp folder, installs the cancel trigger, runs the tool
    /// read-only in the folder, then clears the trigger. The folder is removed
    /// when its handle drops at the end of this scope.
    async fn run_in_temp_folder(
        &self,
        tool_id: String,
        prompt: String,
    ) -> Result<String, SuggestContextsError> {
        let temp_directory =
            self.temp_workspace
                .create()
                .await
                .map_err(|error| SuggestContextsError::Failed {
                    reason: error.reason,
                })?;

        let (trigger, signal) = cancel_channel();
        self.install_cancel(trigger);

        let spec = AiAssistRunSpec {
            tool_id,
            working_directory: temp_directory.path(),
            prompt,
        };
        let outcome = self.runner.run(spec, signal).await;

        // The run is over: drop the trigger whether it finished, timed out or
        // was cancelled, so a later cancel does not fire a dead signal.
        let _ = self.take_in_flight_cancel();

        Ok(outcome?)
    }

    /// Names each raw draft's branch under the project's policy, keeping only
    /// the ones that could be named.
    fn name_branches(
        &self,
        project_tag: &str,
        raw_drafts: &[RawContextDraft],
    ) -> Vec<ContextDraft> {
        raw_drafts
            .iter()
            .filter_map(|draft| self.name_one(project_tag, draft))
            .collect()
    }

    fn name_one(&self, project_tag: &str, draft: &RawContextDraft) -> Option<ContextDraft> {
        let branch_name = self
            .branch_naming
            .name_branch(project_tag, &draft.branch_name)
            .ok()?;
        Some(ContextDraft {
            context_name: draft.context_name.clone(),
            branch_name,
            prompt: draft.prompt.clone(),
        })
    }

    fn install_cancel(&self, trigger: CancelTrigger) {
        *self.lock_cancel() = Some(trigger);
    }

    fn take_in_flight_cancel(&self) -> Option<CancelTrigger> {
        self.lock_cancel().take()
    }

    fn lock_cancel(&self) -> std::sync::MutexGuard<'_, Option<CancelTrigger>> {
        self.in_flight_cancel
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_assist::domain::ai_assist_runner::{AiAssistRunError, CancelSignal};
    use crate::ai_assist::domain::ports::{
        BranchNamingUnavailable, TempDirectory, TempWorkspaceUnavailable,
    };
    use async_trait::async_trait;
    use std::sync::Mutex as StdMutex;

    const PROJECT_TAG: &str = "acme";
    const INSTRUCTION: &str = "Add login and configure CI";
    const KIRO_TOOL_ID: &str = "kiro";
    const TEMP_PATH: &str = "/tmp/ai-assist-run";
    const TWO_DRAFTS: &str = r#"[{"contextName":"Add login","branchName":"add-login","preprompt":"do login"},{"contextName":"Configure CI","branchName":"configure-ci"}]"#;

    /// A runner scripted with one outcome, recording the spec it was asked to
    /// run so a test can assert the run was read-only in the temp folder.
    struct ScriptedRunner {
        outcome: Result<String, AiAssistRunError>,
        recorded_spec: StdMutex<Option<AiAssistRunSpec>>,
    }

    impl ScriptedRunner {
        fn returning(answer: &str) -> Self {
            Self {
                outcome: Ok(answer.to_string()),
                recorded_spec: StdMutex::new(None),
            }
        }

        fn failing(error: AiAssistRunError) -> Self {
            Self {
                outcome: Err(error),
                recorded_spec: StdMutex::new(None),
            }
        }

        fn ran_spec(&self) -> Option<AiAssistRunSpec> {
            self.recorded_spec.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl AiAssistRunner for ScriptedRunner {
        async fn run(
            &self,
            spec: AiAssistRunSpec,
            _cancel: CancelSignal,
        ) -> Result<String, AiAssistRunError> {
            *self.recorded_spec.lock().unwrap() = Some(spec);
            self.outcome.clone()
        }
    }

    struct FixedDefaultTool {
        tool_id: String,
    }

    impl DefaultAiTool for FixedDefaultTool {
        fn tool_id(&self) -> String {
            self.tool_id.clone()
        }
    }

    /// Names a branch by slugifying the proposal, failing for a name whose slug
    /// would be empty (no letters or digits) — the realistic unnameable case.
    struct SlugBranchNaming;

    impl ProjectBranchNaming for SlugBranchNaming {
        fn name_branch(
            &self,
            _project_tag: &str,
            proposed_name: &str,
        ) -> Result<String, BranchNamingUnavailable> {
            let slug: String = proposed_name
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() {
                        character.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .trim_matches('-')
                .to_string();
            if slug.is_empty() {
                Err(BranchNamingUnavailable)
            } else {
                Ok(format!("feature/{slug}"))
            }
        }
    }

    /// Fails to name any branch, so every draft is dropped.
    struct NoBranchNaming;

    impl ProjectBranchNaming for NoBranchNaming {
        fn name_branch(
            &self,
            _project_tag: &str,
            _proposed_name: &str,
        ) -> Result<String, BranchNamingUnavailable> {
            Err(BranchNamingUnavailable)
        }
    }

    struct RecordingTempDirectory {
        cleaned: Arc<StdMutex<bool>>,
    }

    impl TempDirectory for RecordingTempDirectory {
        fn path(&self) -> String {
            TEMP_PATH.to_string()
        }
    }

    impl Drop for RecordingTempDirectory {
        fn drop(&mut self) {
            *self.cleaned.lock().unwrap() = true;
        }
    }

    /// Records that a temp folder was created and, through the handle's drop,
    /// that it was cleaned up.
    struct RecordingTempWorkspace {
        created: Arc<StdMutex<bool>>,
        cleaned: Arc<StdMutex<bool>>,
    }

    impl RecordingTempWorkspace {
        fn new() -> Self {
            Self {
                created: Arc::new(StdMutex::new(false)),
                cleaned: Arc::new(StdMutex::new(false)),
            }
        }
    }

    #[async_trait]
    impl TempWorkspace for RecordingTempWorkspace {
        async fn create(&self) -> Result<Box<dyn TempDirectory>, TempWorkspaceUnavailable> {
            *self.created.lock().unwrap() = true;
            Ok(Box::new(RecordingTempDirectory {
                cleaned: Arc::clone(&self.cleaned),
            }))
        }
    }

    fn service_with(
        runner: Arc<dyn AiAssistRunner>,
        branch_naming: Arc<dyn ProjectBranchNaming>,
        temp_workspace: Arc<RecordingTempWorkspace>,
    ) -> AiAssistService {
        AiAssistService::new(
            runner,
            Arc::new(FixedDefaultTool {
                tool_id: KIRO_TOOL_ID.to_string(),
            }),
            branch_naming,
            temp_workspace,
        )
    }

    #[tokio::test]
    async fn names_each_draft_branch_under_the_policy() {
        let service = service_with(
            Arc::new(ScriptedRunner::returning(TWO_DRAFTS)),
            Arc::new(SlugBranchNaming),
            Arc::new(RecordingTempWorkspace::new()),
        );

        let drafts = service
            .suggest_contexts(PROJECT_TAG, INSTRUCTION)
            .await
            .unwrap();

        assert_eq!(
            drafts,
            vec![
                ContextDraft {
                    context_name: "Add login".to_string(),
                    branch_name: "feature/add-login".to_string(),
                    prompt: Some("do login".to_string()),
                },
                ContextDraft {
                    context_name: "Configure CI".to_string(),
                    branch_name: "feature/configure-ci".to_string(),
                    prompt: None,
                },
            ]
        );
    }

    #[tokio::test]
    async fn drops_drafts_whose_branch_cannot_be_named() {
        let one_nameable_one_not = r#"[{"contextName":"Add login","branchName":"add-login"},{"contextName":"!!!","branchName":"!!!"}]"#;
        let service = service_with(
            Arc::new(ScriptedRunner::returning(one_nameable_one_not)),
            Arc::new(SlugBranchNaming),
            Arc::new(RecordingTempWorkspace::new()),
        );

        let drafts = service
            .suggest_contexts(PROJECT_TAG, INSTRUCTION)
            .await
            .unwrap();

        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].context_name, "Add login");
    }

    #[tokio::test]
    async fn zero_named_drafts_is_no_valid_entries() {
        let service = service_with(
            Arc::new(ScriptedRunner::returning(TWO_DRAFTS)),
            Arc::new(NoBranchNaming),
            Arc::new(RecordingTempWorkspace::new()),
        );

        let result = service.suggest_contexts(PROJECT_TAG, INSTRUCTION).await;

        assert_eq!(result, Err(SuggestContextsError::NoValidEntries));
    }

    #[tokio::test]
    async fn a_blank_instruction_is_rejected_without_running() {
        let runner = Arc::new(ScriptedRunner::returning(TWO_DRAFTS));
        let service = service_with(
            Arc::clone(&runner) as Arc<dyn AiAssistRunner>,
            Arc::new(SlugBranchNaming),
            Arc::new(RecordingTempWorkspace::new()),
        );

        let result = service.suggest_contexts(PROJECT_TAG, "   ").await;

        assert_eq!(result, Err(SuggestContextsError::EmptyInstruction));
        assert_eq!(runner.ran_spec(), None);
    }

    #[tokio::test]
    async fn runs_read_only_in_the_temp_folder() {
        let runner = Arc::new(ScriptedRunner::returning(TWO_DRAFTS));
        let temp_workspace = Arc::new(RecordingTempWorkspace::new());
        let service = service_with(
            Arc::clone(&runner) as Arc<dyn AiAssistRunner>,
            Arc::new(SlugBranchNaming),
            Arc::clone(&temp_workspace),
        );

        service
            .suggest_contexts(PROJECT_TAG, INSTRUCTION)
            .await
            .unwrap();

        let spec = runner.ran_spec().expect("the runner was asked to run");
        assert_eq!(spec.tool_id, KIRO_TOOL_ID);
        assert_eq!(spec.working_directory, TEMP_PATH);
        assert!(*temp_workspace.created.lock().unwrap());
        // The run is over and the service dropped the handle, cleaning it up.
        assert!(*temp_workspace.cleaned.lock().unwrap());
    }

    #[tokio::test]
    async fn each_parse_error_surfaces() {
        let cases = [
            ("I cannot.", SuggestContextsError::NoListFound),
            (r#"[{ "contextName": }]"#, SuggestContextsError::Malformed),
            (
                r#"[{"branchName":"x"}]"#,
                SuggestContextsError::NoValidEntries,
            ),
        ];
        for (answer, expected) in cases {
            let service = service_with(
                Arc::new(ScriptedRunner::returning(answer)),
                Arc::new(SlugBranchNaming),
                Arc::new(RecordingTempWorkspace::new()),
            );

            let result = service.suggest_contexts(PROJECT_TAG, INSTRUCTION).await;

            assert_eq!(result, Err(expected), "{answer}");
        }
    }

    #[tokio::test]
    async fn a_cancelled_run_surfaces_as_cancelled() {
        let service = service_with(
            Arc::new(ScriptedRunner::failing(AiAssistRunError::Cancelled)),
            Arc::new(SlugBranchNaming),
            Arc::new(RecordingTempWorkspace::new()),
        );

        let result = service.suggest_contexts(PROJECT_TAG, INSTRUCTION).await;

        assert_eq!(result, Err(SuggestContextsError::Cancelled));
    }

    #[tokio::test]
    async fn a_timed_out_run_surfaces_as_timed_out() {
        let service = service_with(
            Arc::new(ScriptedRunner::failing(AiAssistRunError::TimedOut)),
            Arc::new(SlugBranchNaming),
            Arc::new(RecordingTempWorkspace::new()),
        );

        let result = service.suggest_contexts(PROJECT_TAG, INSTRUCTION).await;

        assert_eq!(result, Err(SuggestContextsError::TimedOut));
    }

    #[tokio::test]
    async fn a_failed_run_carries_its_reason() {
        let service = service_with(
            Arc::new(ScriptedRunner::failing(AiAssistRunError::Failed {
                reason: "no binary".to_string(),
            })),
            Arc::new(SlugBranchNaming),
            Arc::new(RecordingTempWorkspace::new()),
        );

        let result = service.suggest_contexts(PROJECT_TAG, INSTRUCTION).await;

        assert_eq!(
            result,
            Err(SuggestContextsError::Failed {
                reason: "no binary".to_string(),
            })
        );
    }
}
