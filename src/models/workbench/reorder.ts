import type { PlacementView } from "@/bindings";
import type { PlacementGroups } from "./placementGroups";

/** The placements a drag can reorder within: pinned among pinned, shown among shown. */
type ReorderablePlacement = "pinned" | "shown";
const REORDERABLE_PLACEMENTS: ReorderablePlacement[] = ["pinned", "shown"];

/**
 * Moves `activeTag` to where `overTag` sits in `orderedTags`, returning a new
 * array. A no-op (same tag, or either tag missing) returns the original order
 * unchanged so callers can skip the save.
 */
export function reorderWithinGroup(
	orderedTags: string[],
	activeTag: string,
	overTag: string,
): string[] {
	if (activeTag === overTag) return orderedTags;

	const fromIndex = orderedTags.indexOf(activeTag);
	const toIndex = orderedTags.indexOf(overTag);
	if (fromIndex === -1 || toIndex === -1) return orderedTags;

	const reordered = [...orderedTags];
	reordered.splice(fromIndex, 1);
	reordered.splice(toIndex, 0, activeTag);
	return reordered;
}

/** The reorder a drag should trigger: a group and that group's new tag order. */
export type ReorderDecision = {
	placement: PlacementView;
	orderedProjectTags: string[];
};

/** The reorderable groups keyed by their placement, each as a list of tags. */
function reorderableTags(groups: PlacementGroups): Record<ReorderablePlacement, string[]> {
	return {
		pinned: groups.pinned.map((project) => project.tag),
		shown: groups.shown.map((project) => project.tag),
	};
}

/**
 * Decides what a drop should do: reorder within a single group, or nothing.
 * Returns null for a drop onto itself, a drop with no target, or a cross-group
 * drop (moving between pinned and shown is Pin/Unpin, not a drag).
 */
export function decideReorder(
	groups: PlacementGroups,
	activeTag: string,
	overTag: string | null,
): ReorderDecision | null {
	if (!overTag || activeTag === overTag) return null;

	const tagsByPlacement = reorderableTags(groups);
	for (const placement of REORDERABLE_PLACEMENTS) {
		const orderedTags = tagsByPlacement[placement];
		const sharesGroup = orderedTags.includes(activeTag) && orderedTags.includes(overTag);
		if (!sharesGroup) continue;

		const orderedProjectTags = reorderWithinGroup(orderedTags, activeTag, overTag);
		if (orderedProjectTags === orderedTags) return null;
		return { placement, orderedProjectTags };
	}

	return null;
}
