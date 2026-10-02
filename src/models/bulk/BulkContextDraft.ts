import type { BranchNamingPolicy } from "@/models/BranchNaming";

/**
 * A single context the bulk creator proposes from one high-level instruction.
 *
 * `contextName` and `branchName` are always present. `branchName` has already
 * been forced through the project's {@link BranchNamingPolicy}, so it is a
 * git-safe ref. `preprompt` is the text pre-filled into the context's Kiro
 * input the first time it is opened; it is absent when nothing relevant could
 * be generated for that context.
 */
export type BulkContextDraft = {
	contextName: string;
	branchName: string;
	preprompt?: string;
};

/** Outcome of parsing a headless Kiro run into bulk context drafts. */
export type ParseBulkDraftsResult =
	| { ok: true; drafts: BulkContextDraft[] }
	| { ok: false; error: BulkDraftParseError };

export type BulkDraftParseError =
	| "no-json-found"
	| "invalid-json"
	| "not-an-array"
	| "no-valid-rows";

/** Shape the model is asked to emit for each context, before normalization. */
type RawDraftRow = {
	contextName?: unknown;
	branchName?: unknown;
	preprompt?: unknown;
};

function isNonEmptyString(value: unknown): value is string {
	return typeof value === "string" && value.trim().length > 0;
}

/**
 * Extracts the first JSON array found in free-form model output. The headless
 * run may wrap the array in prose or a ```json fence, so we scan for the first
 * balanced top-level `[...]` rather than trusting the whole string to be JSON.
 */
function extractJsonArray(rawOutput: string): string | null {
	const start = rawOutput.indexOf("[");
	if (start === -1) return null;

	let depth = 0;
	let inString = false;
	let escaped = false;

	for (let index = start; index < rawOutput.length; index += 1) {
		const character = rawOutput[index];

		if (inString) {
			if (escaped) {
				escaped = false;
			} else if (character === "\\") {
				escaped = true;
			} else if (character === '"') {
				inString = false;
			}
			continue;
		}

		if (character === '"') {
			inString = true;
		} else if (character === "[") {
			depth += 1;
		} else if (character === "]") {
			depth -= 1;
			if (depth === 0) {
				return rawOutput.slice(start, index + 1);
			}
		}
	}

	return null;
}

/**
 * Normalizes one raw row into a {@link BulkContextDraft}, forcing the branch
 * name through the policy. A row is dropped (returns null) when it lacks a
 * usable context name or the branch name slugifies to empty. The context name
 * seeds the branch name whenever the model omitted its own `branchName`.
 */
function normalizeRow(row: RawDraftRow, branchNaming: BranchNamingPolicy): BulkContextDraft | null {
	if (!isNonEmptyString(row.contextName)) return null;

	const contextName = row.contextName.trim();
	const branchSeed = isNonEmptyString(row.branchName) ? row.branchName : contextName;
	const branchName = branchNaming.composeBranchName(branchSeed, branchNaming.defaultPrefix);
	if (branchName === "") return null;

	const draft: BulkContextDraft = { contextName, branchName };
	if (isNonEmptyString(row.preprompt)) {
		draft.preprompt = row.preprompt.trim();
	}
	return draft;
}

/**
 * Parses the raw stdout of a headless bulk-generation run into validated
 * drafts. Branch names are forced through `branchNaming` so they are always
 * valid git refs regardless of what the model returned. Invalid rows are
 * dropped; the parse only fails when no JSON array is present, the JSON is
 * malformed, it is not an array, or no row survives validation.
 */
export function parseBulkContextDrafts(
	rawOutput: string,
	branchNaming: BranchNamingPolicy,
): ParseBulkDraftsResult {
	const jsonText = extractJsonArray(rawOutput);
	if (jsonText === null) return { ok: false, error: "no-json-found" };

	let parsed: unknown;
	try {
		parsed = JSON.parse(jsonText);
	} catch {
		return { ok: false, error: "invalid-json" };
	}

	if (!Array.isArray(parsed)) return { ok: false, error: "not-an-array" };

	const drafts = parsed
		.filter((row): row is RawDraftRow => typeof row === "object" && row !== null)
		.map((row) => normalizeRow(row, branchNaming))
		.filter((draft): draft is BulkContextDraft => draft !== null);

	if (drafts.length === 0) return { ok: false, error: "no-valid-rows" };

	return { ok: true, drafts };
}
