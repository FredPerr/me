type PullRequestDescriptionPromptInput = {
	/** Human-readable name of the work item (context) the branch belongs to. */
	contextName: string;
	/** Repository the pull request targets. */
	repositoryName: string;
	/** Unified diff of the branch against its base. */
	diff: string;
	/** Contents of the repository's `.github` PR template, if one exists. */
	template?: string;
};

/**
 * Build the prompt that drives a headless Kiro run to write a pull request
 * description from a diff. The output is meant to be dropped straight into the
 * description field, so the prompt forbids any wrapping prose or code fences.
 *
 * The description is intentionally terse: a short, scannable bullet list of
 * what changed, with a why only when it is not obvious from the what. When the
 * repository ships a PR template, the agent fills that template instead of
 * inventing its own structure.
 */
export function buildPullRequestDescriptionPrompt(
	input: PullRequestDescriptionPromptInput,
): string {
	const lines = [
		"You are writing the description for a GitHub pull request.",
		`The changes belong to the work item "${input.contextName}" in the "${input.repositoryName}" repository.`,
		"",
		"Write a crisp, scannable description:",
		"- Lead with a short bullet list of what was done, one bullet per change.",
		"- Keep each bullet to a single line. Favor concrete, specific wording.",
		"- Add a brief why only when the reason is not obvious from the change itself.",
		"- Do not restate the diff line by line, do not pad with filler, do not add a title.",
		"",
	];

	if (input.template && input.template.trim().length > 0) {
		lines.push(
			"The repository provides the pull request template below. Fill it in,",
			"keeping its headings and structure, and remove any template comments or",
			"unused optional sections. Keep the same terse, bulleted style.",
			"",
			"--- PULL REQUEST TEMPLATE ---",
			input.template.trim(),
			"--- END TEMPLATE ---",
			"",
		);
	}

	lines.push(
		"Respond with ONLY the final Markdown description. No preamble, no closing",
		"remarks, no surrounding code fences.",
		"",
		"--- DIFF ---",
		input.diff,
		"--- END DIFF ---",
	);

	return lines.join("\n");
}
