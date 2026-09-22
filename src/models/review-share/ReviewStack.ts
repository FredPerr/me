/**
 * Ordering logic for a set of pull requests shared for review.
 *
 * A stack of PRs is not strictly a tree: a PR can depend on more than one
 * other PR (e.g. an integration PR that needs two features merged first), so
 * the dependency relation is a directed acyclic graph (DAG). To produce a
 * flat, shareable message we topologically sort the graph so a PR never
 * appears before something it depends on, while otherwise preserving the
 * manual order the user arranged.
 */

/** A pull request the user selected to include in the review message. */
export type ReviewItem = {
	/** Stable identifier within the selection (PR number as string). */
	id: string;
	number: number;
	title: string;
	url: string;
	/** Ids of the PRs this one depends on (must be merged/reviewed first). */
	dependsOn: string[];
};

/** An item placed in the final order, with its stack depth for indentation. */
export type OrderedReviewItem = {
	item: ReviewItem;
	/** How deep in the dependency chain this item sits (0 = no dependencies). */
	depth: number;
};

export type OrderResult =
	| { ok: true; ordered: OrderedReviewItem[] }
	| { ok: false; error: "cycle"; cycleIds: string[] };

/**
 * Order the items so every dependency comes before the items that depend on
 * it. Among items with satisfied dependencies, the original `items` order is
 * preserved (a stable topological sort). Returns a cycle error if the
 * dependencies cannot be linearized.
 */
export function orderReviewItems(items: ReviewItem[]): OrderResult {
	const byId = new Map(items.map((item) => [item.id, item]));

	// Only consider dependencies that point at other included items; a
	// dependency on a PR the user did not select is simply ignored.
	const dependenciesOf = (item: ReviewItem): string[] =>
		item.dependsOn.filter((dependencyId) => byId.has(dependencyId));

	const placed = new Set<string>();
	const ordered: OrderedReviewItem[] = [];

	const remaining = () => items.filter((item) => !placed.has(item.id));

	while (placed.size < items.length) {
		// Place the first item (in manual order) whose dependencies are all
		// already placed. Preserving manual order keeps the user's arrangement.
		const next = remaining().find((item) =>
			dependenciesOf(item).every((dependencyId) => placed.has(dependencyId)),
		);

		if (!next) {
			// No item is placeable: the unplaced items form at least one cycle.
			return { ok: false, error: "cycle", cycleIds: remaining().map((item) => item.id) };
		}

		placed.add(next.id);
		ordered.push({ item: next, depth: 0 });
	}

	return { ok: true, ordered: assignDepths(ordered, dependenciesOf) };
}

/**
 * Compute each item's stack depth as one more than the deepest dependency it
 * has among the included items. Items are processed in placed order, so every
 * dependency's depth is known before its dependents'.
 */
function assignDepths(
	ordered: OrderedReviewItem[],
	dependenciesOf: (item: ReviewItem) => string[],
): OrderedReviewItem[] {
	const depthById = new Map<string, number>();

	return ordered.map(({ item }) => {
		const dependencyDepths = dependenciesOf(item).map(
			(dependencyId) => depthById.get(dependencyId) ?? 0,
		);
		const depth = dependencyDepths.length === 0 ? 0 : Math.max(...dependencyDepths) + 1;
		depthById.set(item.id, depth);
		return { item, depth };
	});
}

/** The minimal PR shape needed to infer stack dependencies from branches. */
export type BranchedPullRequest = {
	id: string;
	headBranch: string;
	baseBranch: string;
};

/**
 * Infer stack dependencies from branch relationships. In a PR stack, a PR is
 * opened with its base branch set to the head branch of the PR beneath it, so
 * PR X depends on PR Y when `X.baseBranch === Y.headBranch`. A PR whose base is
 * not the head of any other included PR is a stack root (it targets the trunk).
 *
 * Returns a map of PR id to the ids it depends on. Multiple PRs sharing a head
 * branch is unexpected, but if it happens every match is recorded.
 */
export function inferDependencies(pullRequests: BranchedPullRequest[]): Record<string, string[]> {
	const idsByHeadBranch = new Map<string, string[]>();
	for (const pullRequest of pullRequests) {
		const existing = idsByHeadBranch.get(pullRequest.headBranch) ?? [];
		existing.push(pullRequest.id);
		idsByHeadBranch.set(pullRequest.headBranch, existing);
	}

	const dependencies: Record<string, string[]> = {};
	for (const pullRequest of pullRequests) {
		const parents = idsByHeadBranch.get(pullRequest.baseBranch) ?? [];
		// A PR never depends on itself, even if base and head somehow match.
		dependencies[pullRequest.id] = parents.filter((parentId) => parentId !== pullRequest.id);
	}

	return dependencies;
}

/**
 * Move the item at `fromIndex` to `toIndex`, returning a new array. Out-of-range
 * indices are clamped; the original array is never mutated.
 */
export function moveItem<T>(items: T[], fromIndex: number, toIndex: number): T[] {
	if (fromIndex < 0 || fromIndex >= items.length) return items;
	const clampedTo = Math.max(0, Math.min(items.length - 1, toIndex));
	if (fromIndex === clampedTo) return items;

	const copy = [...items];
	const [moved] = copy.splice(fromIndex, 1);
	copy.splice(clampedTo, 0, moved);
	return copy;
}
