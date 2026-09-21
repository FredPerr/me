const NO_MATCH = -1;
const CONSECUTIVE_BONUS = 8;
const START_OF_STRING_BONUS = 12;
const AFTER_SEPARATOR_BONUS = 6;
const LEADING_GAP_PENALTY = 1;
const MAX_LEADING_GAP_PENALTY = 3;
const SEPARATORS = new Set(["/", "-", "_", ".", " "]);

/**
 * Fuzzy subsequence matcher. Returns a relevance score when every character of
 * `query` appears in `target` in order (case-insensitive), or {@link NO_MATCH}
 * (-1) when it does not match. Higher scores are more relevant: consecutive
 * characters, matches at the start of the string, and matches right after a
 * separator (e.g. "/") all score higher.
 */
export function fuzzyMatch(target: string, query: string): number {
	const trimmedQuery = query.trim().toLowerCase();
	if (trimmedQuery === "") return 0;

	const haystack = target.toLowerCase();

	let score = 0;
	let queryIndex = 0;
	let previousMatchIndex = NO_MATCH;

	for (let index = 0; index < haystack.length && queryIndex < trimmedQuery.length; index++) {
		if (haystack[index] !== trimmedQuery[queryIndex]) continue;

		const atStart = index === 0;
		const afterSeparator = index > 0 && SEPARATORS.has(haystack[index - 1]);

		if (atStart) {
			score += START_OF_STRING_BONUS;
		} else if (afterSeparator) {
			score += AFTER_SEPARATOR_BONUS;
		}

		if (previousMatchIndex === index - 1) {
			score += CONSECUTIVE_BONUS;
		} else if (previousMatchIndex === NO_MATCH && !atStart && !afterSeparator) {
			score -= Math.min(index * LEADING_GAP_PENALTY, MAX_LEADING_GAP_PENALTY);
		}

		previousMatchIndex = index;
		queryIndex++;
	}

	if (queryIndex < trimmedQuery.length) return NO_MATCH;

	return score;
}

export function isFuzzyMatch(target: string, query: string): boolean {
	return fuzzyMatch(target, query) !== NO_MATCH;
}
