import { describe, expect, it } from "vitest";
import { parseBulkContextDrafts } from "./BulkContextDraft";

type RawRow = { contextName?: unknown; branchName?: unknown; preprompt?: unknown };

function wrapAsOutput(rows: RawRow[]): string {
	return `Here are the contexts:\n${JSON.stringify(rows)}\nDone.`;
}

describe("parseBulkContextDrafts", () => {
	it("extracts a JSON array embedded in surrounding prose", () => {
		const output = wrapAsOutput([{ contextName: "Right-size EC2", branchName: "right-size-ec2" }]);

		const result = parseBulkContextDrafts(output);

		expect(result.ok).toBe(true);
		if (result.ok) {
			expect(result.drafts).toEqual([
				{ contextName: "Right-size EC2", branchName: "right-size-ec2" },
			]);
		}
	});

	it("keeps the preprompt when present and trims it", () => {
		const output = JSON.stringify([
			{
				contextName: "Delete volumes",
				branchName: "delete-volumes",
				preprompt: "  do the thing  ",
			},
		]);

		const result = parseBulkContextDrafts(output);

		expect(result.ok).toBe(true);
		if (result.ok) expect(result.drafts[0].preprompt).toBe("do the thing");
	});

	it("omits the preprompt when it is missing or blank", () => {
		const output = JSON.stringify([
			{ contextName: "A", branchName: "a", preprompt: "   " },
			{ contextName: "B", branchName: "b" },
		]);

		const result = parseBulkContextDrafts(output);

		expect(result.ok).toBe(true);
		if (result.ok) {
			expect(result.drafts[0].preprompt).toBeUndefined();
			expect(result.drafts[1].preprompt).toBeUndefined();
		}
	});

	it("keeps the proposed branch name, trimmed, for the project to name", () => {
		const output = JSON.stringify([{ contextName: "New API", branchName: "  New API!!  " }]);

		const result = parseBulkContextDrafts(output);

		expect(result.ok).toBe(true);
		if (result.ok) expect(result.drafts[0].branchName).toBe("New API!!");
	});

	it("seeds the branch name from the context name when branchName is missing", () => {
		const output = JSON.stringify([{ contextName: "Clean Up Logs" }]);

		const result = parseBulkContextDrafts(output);

		expect(result.ok).toBe(true);
		if (result.ok) expect(result.drafts[0].branchName).toBe("Clean Up Logs");
	});

	it("drops rows without a usable context name", () => {
		const output = JSON.stringify([
			{ branchName: "orphan" },
			{ contextName: "Valid", branchName: "valid" },
		]);

		const result = parseBulkContextDrafts(output);

		expect(result.ok).toBe(true);
		if (result.ok) {
			expect(result.drafts).toHaveLength(1);
			expect(result.drafts[0].contextName).toBe("Valid");
		}
	});

	it("returns no-json-found when there is no array", () => {
		const result = parseBulkContextDrafts("I could not do that.");

		expect(result).toEqual({ ok: false, error: "no-json-found" });
	});

	it("returns invalid-json when the array is malformed", () => {
		const result = parseBulkContextDrafts('[{ "contextName": }]');

		expect(result).toEqual({ ok: false, error: "invalid-json" });
	});

	it("returns no-valid-rows when every row is invalid", () => {
		const output = JSON.stringify([{ branchName: "x" }, { contextName: "" }]);

		const result = parseBulkContextDrafts(output);

		expect(result).toEqual({ ok: false, error: "no-valid-rows" });
	});
});
