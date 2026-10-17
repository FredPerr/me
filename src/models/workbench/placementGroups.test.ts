import { describe, expect, it } from "vitest";
import type { PlacementView, ProjectView, WorkbenchEntryView } from "@/bindings";
import { buildProject } from "@/test-utils/workspaceBuilders";
import {
	groupProjectsByPlacement,
	initialVisibleTag,
	isVisibleTag,
	visibleProjects,
} from "./placementGroups";

const PINNED_TAG = "api";
const SHOWN_TAG = "website";
const HIDDEN_TAG = "legacy";
const SECOND_SHOWN_TAG = "docs";
const UNKNOWN_TAG = "ghost";

function buildEntry(projectTag: string, placement: PlacementView): WorkbenchEntryView {
	return { projectTag, placement };
}

function projectsFor(tags: string[]): ProjectView[] {
	return tags.map((tag) => buildProject({ tag, name: tag }));
}

describe("groupProjectsByPlacement", () => {
	it("splits the entries into pinned, shown and hidden groups", () => {
		const entries = [
			buildEntry(PINNED_TAG, "pinned"),
			buildEntry(SHOWN_TAG, "shown"),
			buildEntry(HIDDEN_TAG, "hidden"),
		];

		const groups = groupProjectsByPlacement(
			entries,
			projectsFor([PINNED_TAG, SHOWN_TAG, HIDDEN_TAG]),
		);

		expect(groups.pinned.map((project) => project.tag)).toEqual([PINNED_TAG]);
		expect(groups.shown.map((project) => project.tag)).toEqual([SHOWN_TAG]);
		expect(groups.hidden.map((project) => project.tag)).toEqual([HIDDEN_TAG]);
	});

	it("keeps each group in the entries' order", () => {
		const entries = [buildEntry(SECOND_SHOWN_TAG, "shown"), buildEntry(SHOWN_TAG, "shown")];

		const groups = groupProjectsByPlacement(entries, projectsFor([SHOWN_TAG, SECOND_SHOWN_TAG]));

		expect(groups.shown.map((project) => project.tag)).toEqual([SECOND_SHOWN_TAG, SHOWN_TAG]);
	});

	it("drops an entry whose project is not loaded", () => {
		const entries = [buildEntry(SHOWN_TAG, "shown"), buildEntry(UNKNOWN_TAG, "shown")];

		const groups = groupProjectsByPlacement(entries, projectsFor([SHOWN_TAG]));

		expect(groups.shown.map((project) => project.tag)).toEqual([SHOWN_TAG]);
	});
});

describe("visibleProjects", () => {
	it("lists pinned projects before shown projects", () => {
		const entries = [
			buildEntry(SHOWN_TAG, "shown"),
			buildEntry(PINNED_TAG, "pinned"),
			buildEntry(HIDDEN_TAG, "hidden"),
		];
		const groups = groupProjectsByPlacement(
			entries,
			projectsFor([PINNED_TAG, SHOWN_TAG, HIDDEN_TAG]),
		);

		expect(visibleProjects(groups).map((project) => project.tag)).toEqual([PINNED_TAG, SHOWN_TAG]);
	});
});

describe("isVisibleTag", () => {
	const groups = groupProjectsByPlacement(
		[buildEntry(SHOWN_TAG, "shown"), buildEntry(HIDDEN_TAG, "hidden")],
		projectsFor([SHOWN_TAG, HIDDEN_TAG]),
	);

	it("is true for a shown project", () => {
		expect(isVisibleTag(groups, SHOWN_TAG)).toBe(true);
	});

	it("is false for a hidden project", () => {
		expect(isVisibleTag(groups, HIDDEN_TAG)).toBe(false);
	});
});

describe("initialVisibleTag", () => {
	const groups = groupProjectsByPlacement(
		[
			buildEntry(PINNED_TAG, "pinned"),
			buildEntry(SHOWN_TAG, "shown"),
			buildEntry(HIDDEN_TAG, "hidden"),
		],
		projectsFor([PINNED_TAG, SHOWN_TAG, HIDDEN_TAG]),
	);

	it("is the last-open project when it is still visible", () => {
		expect(initialVisibleTag(groups, SHOWN_TAG)).toBe(SHOWN_TAG);
	});

	it("falls back to the first pinned project when last-open is null", () => {
		expect(initialVisibleTag(groups, null)).toBe(PINNED_TAG);
	});

	it("falls back to the first pinned project when last-open is hidden", () => {
		expect(initialVisibleTag(groups, HIDDEN_TAG)).toBe(PINNED_TAG);
	});

	it("falls back to the first shown project when nothing is pinned", () => {
		const shownOnly = groupProjectsByPlacement(
			[buildEntry(SHOWN_TAG, "shown")],
			projectsFor([SHOWN_TAG]),
		);

		expect(initialVisibleTag(shownOnly, null)).toBe(SHOWN_TAG);
	});

	it("is null when nothing is visible", () => {
		const hiddenOnly = groupProjectsByPlacement(
			[buildEntry(HIDDEN_TAG, "hidden")],
			projectsFor([HIDDEN_TAG]),
		);

		expect(initialVisibleTag(hiddenOnly, null)).toBe(null);
	});
});
