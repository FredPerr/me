import { describe, expect, it } from "vitest";
import { moveItem, orderReviewItems, type ReviewItem } from "./ReviewStack";

function buildItem(overrides: Partial<ReviewItem> & { id: string }): ReviewItem {
	return {
		number: Number(overrides.id),
		title: `PR ${overrides.id}`,
		url: `https://github.com/acme/repo/pull/${overrides.id}`,
		dependsOn: [],
		...overrides,
	};
}

describe("orderReviewItems", () => {
	it("preserves manual order when there are no dependencies", () => {
		const items = [buildItem({ id: "1" }), buildItem({ id: "2" }), buildItem({ id: "3" })];

		const result = orderReviewItems(items);

		expect(result.ok).toBe(true);
		if (!result.ok) return;
		expect(result.ordered.map((entry) => entry.item.id)).toEqual(["1", "2", "3"]);
		expect(result.ordered.every((entry) => entry.depth === 0)).toBe(true);
	});

	it("places a dependency before the item that depends on it", () => {
		const dependent = buildItem({ id: "1", dependsOn: ["2"] });
		const prerequisite = buildItem({ id: "2" });

		const result = orderReviewItems([dependent, prerequisite]);

		expect(result.ok).toBe(true);
		if (!result.ok) return;
		expect(result.ordered.map((entry) => entry.item.id)).toEqual(["2", "1"]);
	});

	it("assigns increasing depth along a dependency chain", () => {
		const items = [
			buildItem({ id: "1" }),
			buildItem({ id: "2", dependsOn: ["1"] }),
			buildItem({ id: "3", dependsOn: ["2"] }),
		];

		const result = orderReviewItems(items);

		expect(result.ok).toBe(true);
		if (!result.ok) return;
		const depthById = Object.fromEntries(
			result.ordered.map((entry) => [entry.item.id, entry.depth]),
		);
		expect(depthById).toEqual({ "1": 0, "2": 1, "3": 2 });
	});

	it("computes depth from the deepest dependency for a diamond", () => {
		const items = [
			buildItem({ id: "1" }),
			buildItem({ id: "2", dependsOn: ["1"] }),
			buildItem({ id: "3", dependsOn: ["1"] }),
			buildItem({ id: "4", dependsOn: ["2", "3"] }),
		];

		const result = orderReviewItems(items);

		expect(result.ok).toBe(true);
		if (!result.ok) return;
		const depthById = Object.fromEntries(
			result.ordered.map((entry) => [entry.item.id, entry.depth]),
		);
		expect(depthById["4"]).toBe(2);
	});

	it("ignores dependencies pointing at items that are not included", () => {
		const item = buildItem({ id: "1", dependsOn: ["99"] });

		const result = orderReviewItems([item]);

		expect(result.ok).toBe(true);
		if (!result.ok) return;
		expect(result.ordered[0].depth).toBe(0);
	});

	it("reports a cycle when dependencies cannot be linearized", () => {
		const first = buildItem({ id: "1", dependsOn: ["2"] });
		const second = buildItem({ id: "2", dependsOn: ["1"] });

		const result = orderReviewItems([first, second]);

		expect(result.ok).toBe(false);
		if (result.ok) return;
		expect(result.error).toBe("cycle");
		expect(result.cycleIds.sort()).toEqual(["1", "2"]);
	});
});

describe("moveItem", () => {
	it("moves an item later in the list", () => {
		const result = moveItem(["a", "b", "c"], 0, 2);

		expect(result).toEqual(["b", "c", "a"]);
	});

	it("moves an item earlier in the list", () => {
		const result = moveItem(["a", "b", "c"], 2, 0);

		expect(result).toEqual(["c", "a", "b"]);
	});

	it("returns the original array when the source index is out of range", () => {
		const original = ["a", "b"];

		const result = moveItem(original, 5, 0);

		expect(result).toBe(original);
	});

	it("clamps the destination index within bounds", () => {
		const result = moveItem(["a", "b", "c"], 0, 99);

		expect(result).toEqual(["b", "c", "a"]);
	});
});
