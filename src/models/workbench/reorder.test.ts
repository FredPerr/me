import { describe, expect, it } from "vitest";
import type { ProjectView } from "@/bindings";
import { buildProject } from "@/test-utils/workspaceBuilders";
import type { PlacementGroups } from "./placementGroups";
import { decideReorder, reorderWithinGroup } from "./reorder";

const API_TAG = "api";
const WEBSITE_TAG = "website";
const MOBILE_TAG = "mobile";
const UNKNOWN_TAG = "ghost";

const SHOWN_ORDER = [API_TAG, WEBSITE_TAG, MOBILE_TAG];

function projectsFor(tags: string[]): ProjectView[] {
	return tags.map((tag) => buildProject({ tag, name: tag }));
}

function buildGroups(overrides?: Partial<PlacementGroups>): PlacementGroups {
	return {
		pinned: [],
		shown: [],
		hidden: [],
		...overrides,
	};
}

describe("reorderWithinGroup", () => {
	it("moves an item earlier when dropped before its target", () => {
		const result = reorderWithinGroup(SHOWN_ORDER, MOBILE_TAG, API_TAG);

		expect(result).toEqual([MOBILE_TAG, API_TAG, WEBSITE_TAG]);
	});

	it("moves an item later when dropped after its target", () => {
		const result = reorderWithinGroup(SHOWN_ORDER, API_TAG, MOBILE_TAG);

		expect(result).toEqual([WEBSITE_TAG, MOBILE_TAG, API_TAG]);
	});

	it("returns the same order when active and over are the same", () => {
		const result = reorderWithinGroup(SHOWN_ORDER, API_TAG, API_TAG);

		expect(result).toBe(SHOWN_ORDER);
	});

	it("returns the same order when a tag is unknown", () => {
		const result = reorderWithinGroup(SHOWN_ORDER, API_TAG, UNKNOWN_TAG);

		expect(result).toBe(SHOWN_ORDER);
	});
});

describe("decideReorder", () => {
	it("reorders within the shown group", () => {
		const groups = buildGroups({ shown: projectsFor(SHOWN_ORDER) });

		const decision = decideReorder(groups, API_TAG, MOBILE_TAG);

		expect(decision).toEqual({
			placement: "shown",
			orderedProjectTags: [WEBSITE_TAG, MOBILE_TAG, API_TAG],
		});
	});

	it("reorders within the pinned group", () => {
		const groups = buildGroups({ pinned: projectsFor([API_TAG, WEBSITE_TAG]) });

		const decision = decideReorder(groups, WEBSITE_TAG, API_TAG);

		expect(decision).toEqual({
			placement: "pinned",
			orderedProjectTags: [WEBSITE_TAG, API_TAG],
		});
	});

	it("returns null for a cross-group drop", () => {
		const groups = buildGroups({
			pinned: projectsFor([API_TAG]),
			shown: projectsFor([WEBSITE_TAG]),
		});

		const decision = decideReorder(groups, API_TAG, WEBSITE_TAG);

		expect(decision).toBeNull();
	});

	it("returns null when there is no target", () => {
		const groups = buildGroups({ shown: projectsFor(SHOWN_ORDER) });

		const decision = decideReorder(groups, API_TAG, null);

		expect(decision).toBeNull();
	});

	it("returns null when dropped onto itself", () => {
		const groups = buildGroups({ shown: projectsFor(SHOWN_ORDER) });

		const decision = decideReorder(groups, API_TAG, API_TAG);

		expect(decision).toBeNull();
	});
});
