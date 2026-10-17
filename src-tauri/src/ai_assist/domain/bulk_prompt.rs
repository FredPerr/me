//! The bulk-context prompt: one instruction in, a prompt asking for a JSON
//! array of contexts out. Ported verbatim from the frontend
//! `bulkContextPrompt.ts` so the tool sees the same instruction it did before
//! the use case moved to Rust.

/// Builds the headless prompt that expands one high-level instruction into a
/// list of context drafts. The text is the frontend prompt, line for line, so
/// the parser can keep matching the same sample outputs.
pub fn build_bulk_context_prompt(instruction: &str) -> String {
    [
        "You are helping divide a larger software task into several smaller work items.",
        "Each work item becomes its own git worktree context with its own branch.",
        "",
        "From the instruction below, produce a list of contexts to create.",
        "",
        "Respond with ONLY a single JSON array and nothing else. No prose, no",
        "explanation, no markdown fences. Each array element must be an object with:",
        "  - \"contextName\": a short, human-readable name for the work item (required)",
        "  - \"branchName\": a short slug for the git branch, lowercase and hyphenated (required)",
        "  - \"preprompt\": a focused prompt an AI coding agent could run to complete",
        "      this specific work item (optional — omit the field entirely if you",
        "      cannot write a genuinely useful, specific prompt for this item)",
        "",
        "Keep each context tightly scoped to one unit of work. Do not invent items",
        "that are not implied by the instruction.",
        "",
        "Instruction:",
        instruction.trim(),
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTRUCTION: &str = "Add login and configure CI";

    #[test]
    fn places_the_trimmed_instruction_after_the_instruction_label() {
        let prompt = build_bulk_context_prompt("  Add login and configure CI  ");

        assert!(prompt.ends_with(&format!("Instruction:\n{INSTRUCTION}")));
    }

    #[test]
    fn asks_for_a_single_json_array_and_nothing_else() {
        let prompt = build_bulk_context_prompt(INSTRUCTION);

        assert!(prompt.contains("Respond with ONLY a single JSON array and nothing else."));
    }

    #[test]
    fn describes_the_three_object_fields() {
        let prompt = build_bulk_context_prompt(INSTRUCTION);

        assert!(prompt.contains("\"contextName\""));
        assert!(prompt.contains("\"branchName\""));
        assert!(prompt.contains("\"preprompt\""));
    }
}
