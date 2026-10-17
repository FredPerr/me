//! The pull-request prompt: a repository's diff (and optional PR template) in,
//! a prompt asking for a title and a description out.
//!
//! Ported from the frontend `pullRequestDescriptionPrompt.ts`. The description
//! half keeps the exact wording it had there (the terse bulleted style, the
//! template section, and the "ONLY the final Markdown description" instruction)
//! so the tool writes the same body it did before the use case moved to Rust.
//!
//! #89 extends it: the use case now returns both a title and a description (the
//! old TS only produced a description). The prompt asks for a single
//! `Title: ...` line first, then the Markdown body — a shape that is trivial to
//! parse apart (see [`parse_suggested_pull_request`]) and degrades gracefully
//! when the model omits the title line.

/// What the prompt is built from: the work item and repository the changes
/// belong to, the diff against the base, and the repository's optional pull
/// request template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestPromptInput {
    /// Human-readable name of the work item (context) the branch belongs to.
    pub context_name: String,
    /// Repository the pull request targets.
    pub repository_name: String,
    /// Unified diff of the branch against its base.
    pub diff: String,
    /// Contents of the repository's pull request template, if one exists.
    pub template: Option<String>,
}

/// The single line the title is asked for on, so the parser can split it from
/// the body deterministically. Kept in sync with
/// [`parse_suggested_pull_request`](super::suggested_pull_request::parse_suggested_pull_request).
pub const TITLE_LINE_PREFIX: &str = "Title:";

/// Builds the headless prompt that writes a pull request title and description
/// from a diff. The description guidance is the frontend prompt, line for line;
/// a title instruction is added on top so the single run yields both halves.
pub fn build_pull_request_prompt(input: &PullRequestPromptInput) -> String {
    let mut lines = vec![
        "You are writing the title and description for a GitHub pull request.".to_string(),
        format!(
            "The changes belong to the work item \"{}\" in the \"{}\" repository.",
            input.context_name, input.repository_name
        ),
        String::new(),
        format!(
            "Start your answer with a single line \"{TITLE_LINE_PREFIX} <a short, \
             specific pull request title>\", then a blank line, then the description."
        ),
        String::new(),
        "Write a crisp, scannable description:".to_string(),
        "- Lead with a short bullet list of what was done, one bullet per change.".to_string(),
        "- Keep each bullet to a single line. Favor concrete, specific wording.".to_string(),
        "- Add a brief why only when the reason is not obvious from the change itself.".to_string(),
        "- Do not restate the diff line by line, do not pad with filler, do not repeat the title."
            .to_string(),
        String::new(),
    ];

    if let Some(template) = template_section(input.template.as_deref()) {
        lines.extend(template);
    }

    lines.extend([
        "Respond with ONLY the title line and the final Markdown description. No preamble, no"
            .to_string(),
        "closing remarks, no surrounding code fences.".to_string(),
        String::new(),
        "--- DIFF ---".to_string(),
        input.diff.clone(),
        "--- END DIFF ---".to_string(),
    ]);

    lines.join("\n")
}

/// The template block, when the repository ships a non-blank template. Kept
/// verbatim from the frontend so a templated description is filled the same way.
fn template_section(template: Option<&str>) -> Option<Vec<String>> {
    let template = template?.trim();
    if template.is_empty() {
        return None;
    }
    Some(vec![
        "The repository provides the pull request template below. Fill it in,".to_string(),
        "keeping its headings and structure, and remove any template comments or".to_string(),
        "unused optional sections. Keep the same terse, bulleted style.".to_string(),
        String::new(),
        "--- PULL REQUEST TEMPLATE ---".to_string(),
        template.to_string(),
        "--- END TEMPLATE ---".to_string(),
        String::new(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTEXT_NAME: &str = "Add login";
    const REPOSITORY_NAME: &str = "api";
    const DIFF: &str = "diff --git a/login.ts b/login.ts\n+export const login = () => {}";
    const TEMPLATE: &str = "## What changed\n\n## Why";

    fn input_with_template(template: Option<&str>) -> PullRequestPromptInput {
        PullRequestPromptInput {
            context_name: CONTEXT_NAME.to_string(),
            repository_name: REPOSITORY_NAME.to_string(),
            diff: DIFF.to_string(),
            template: template.map(ToString::to_string),
        }
    }

    #[test]
    fn names_the_work_item_and_repository() {
        let prompt = build_pull_request_prompt(&input_with_template(None));

        assert!(prompt.contains("work item \"Add login\" in the \"api\" repository"));
    }

    #[test]
    fn asks_for_a_title_line_before_the_description() {
        let prompt = build_pull_request_prompt(&input_with_template(None));

        assert!(prompt.contains("\"Title: <a short, specific pull request title>\""));
    }

    #[test]
    fn keeps_the_terse_bulleted_description_guidance() {
        let prompt = build_pull_request_prompt(&input_with_template(None));

        assert!(prompt.contains("Lead with a short bullet list of what was done"));
        assert!(prompt.contains("Add a brief why only when the reason is not obvious"));
    }

    #[test]
    fn forbids_wrapping_prose_and_fences() {
        let prompt = build_pull_request_prompt(&input_with_template(None));

        assert!(
            prompt.contains("Respond with ONLY the title line and the final Markdown description.")
        );
        assert!(prompt.contains("no surrounding code fences."));
    }

    #[test]
    fn places_the_diff_between_the_diff_markers() {
        let prompt = build_pull_request_prompt(&input_with_template(None));

        let diff_section = format!("--- DIFF ---\n{DIFF}\n--- END DIFF ---");
        assert!(prompt.ends_with(&diff_section));
    }

    #[test]
    fn includes_the_trimmed_template_when_one_is_given() {
        let prompt = build_pull_request_prompt(&input_with_template(Some("  ## What changed\n  ")));

        assert!(
            prompt.contains("--- PULL REQUEST TEMPLATE ---\n## What changed\n--- END TEMPLATE ---")
        );
    }

    #[test]
    fn omits_the_template_section_when_there_is_no_template() {
        let prompt = build_pull_request_prompt(&input_with_template(None));

        assert!(!prompt.contains("PULL REQUEST TEMPLATE"));
    }

    #[test]
    fn omits_the_template_section_for_a_blank_template() {
        let prompt = build_pull_request_prompt(&input_with_template(Some("   \n  ")));

        assert!(!prompt.contains("PULL REQUEST TEMPLATE"));
        let _ = TEMPLATE;
    }
}
