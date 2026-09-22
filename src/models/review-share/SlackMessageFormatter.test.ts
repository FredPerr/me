import { describe, expect, it } from "vitest";
import type { OrderedReviewItem, ReviewItem } from "./ReviewStack";
import { formatSlackMessage } from "./SlackMessageFormatter";

function buildOrdered(id: string, depth: number, dependsOn: string[] = []): OrderedReviewItem {
	const item: ReviewItem = {
		id,
		number: Number(id),
		title: `PR ${id}`,
		url: `https://github.com/acme/repo/pull/${id}`,
		dependsOn,
	};
	return { item, depth };
}

const HEADING = "PRs ready for review:";

describe("formatSlackMessage", () => {
	it("returns an empty string when there are no items", () => {
		const result = formatSlackMessage([]);

		expect(result).toBe("");
	});

	it("formats a single PR as a Slack link", () => {
		const result = formatSlackMessage([buildOrdered("1", 0)]);

		expect(result).toBe("<https://github.com/acme/repo/pull/1|#1 PR 1>");
	});

	it("places the heading on the first line", () => {
		const result = formatSlackMessage([buildOrdered("1", 0)], HEADING);

		expect(result.split("\n")[0]).toBe(HEADING);
	});

	it("indents a dependent PR and notes the dependency", () => {
		const ordered = [buildOrdered("1", 0), buildOrdered("2", 1, ["1"])];

		const result = formatSlackMessage(ordered);

		const secondLine = result.split("\n")[1];
		expect(secondLine).toBe("    └ <https://github.com/acme/repo/pull/2|#2 PR 2> (depends on #1)");
	});

	it("omits dependency notes that point outside the included set", () => {
		const result = formatSlackMessage([buildOrdered("1", 0, ["99"])]);

		expect(result).not.toContain("depends on");
	});
});
