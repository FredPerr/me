export function buildBulkContextPrompt(instruction: string): string {
	return [
		"You are helping divide a larger software task into several smaller work items.",
		"Each work item becomes its own git worktree context with its own branch.",
		"",
		"From the instruction below, produce a list of contexts to create.",
		"",
		"Respond with ONLY a single JSON array and nothing else. No prose, no",
		"explanation, no markdown fences. Each array element must be an object with:",
		'  - "contextName": a short, human-readable name for the work item (required)',
		'  - "branchName": a short slug for the git branch, lowercase and hyphenated (required)',
		'  - "preprompt": a focused prompt an AI coding agent could run to complete',
		"      this specific work item (optional — omit the field entirely if you",
		"      cannot write a genuinely useful, specific prompt for this item)",
		"",
		"Keep each context tightly scoped to one unit of work. Do not invent items",
		"that are not implied by the instruction.",
		"",
		"Instruction:",
		instruction.trim(),
	].join("\n");
}
