//! Context drafts: what `suggest_contexts` turns one instruction into.
//!
//! Two shapes live here, one per stage of the pipeline:
//!   - [`RawContextDraft`] is what the parser recovers from the tool's output:
//!     a context name, the branch name the model proposed (or the context name
//!     seeded in its place), and an optional prompt. Its branch name is only a
//!     proposal — it is not yet valid under any project's naming policy.
//!   - [`ContextDraft`] is what the use case returns: the same draft after the
//!     project's branch naming policy has turned the proposal into a valid
//!     branch name. A draft whose branch could not be made valid never becomes
//!     a [`ContextDraft`] — it is dropped.

/// A draft as recovered from the tool's output, before its branch is named by
/// the project's policy. `branch_name` is the model's proposal (or the context
/// name, seeded when the model omitted it), not yet a valid branch name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawContextDraft {
    pub context_name: String,
    pub branch_name: String,
    pub prompt: Option<String>,
}

/// A ready-to-create context draft. `branch_name` is valid under the project's
/// branch naming policy; the use case drops any draft it could not name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextDraft {
    pub context_name: String,
    pub branch_name: String,
    pub prompt: Option<String>,
}
