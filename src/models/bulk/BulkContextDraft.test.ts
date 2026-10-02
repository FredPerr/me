import { describe, expect, it } from "vitest";
import { BranchNamingPolicy } from "@/models/BranchNaming";
import { parseBulkContextDrafts } from "./BulkContextDraft";

const NO_PREFIX_POLICY = BranchNamingPolicy.create([], true);
const REQUIRED_PREFIX_POLICY = BranchNamingPolicy.create(["feature"], false);

type RawRow = { contextName?: unknown; branchName?: unknown; preprompt?: unknown };

function wrapAsOutput(rows: RawRow[]): string {
	return `Here are the contexts:\n${JSON.stringify(rows)}\nDone.`;
}

describe("parseBulkContextDrafts", () => {
	it("extracts a JSON array embedded in surrounding prose", () => {
		const output = wrapAsOutput([{ contextName: "Right-size EC2", branchName: "right-size-ec2" }]);

		const result = parseBulkContextDrafts(output, NO_PREFIX_POLICY);

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

		const result = parseBulkContextDrafts(output, NO_PREFIX_POLICY);

		expect(result.ok).toBe(true);
		if (result.ok) expect(result.drafts[0].preprompt).toBe("do the thing");
	});

	it("omits the preprompt when it is missing or blank", () => {
		const output = JSON.stringify([
			{ contextName: "A", branchName: "a", preprompt: "   " },
			{ contextName: "B", branchName: "b" },
		]);

		const result = parseBulkContextDrafts(output, NO_PREFIX_POLICY);

		expect(result.ok).toBe(true);
		if (result.ok) {
			expect(result.drafts[0].preprompt).toBeUndefined();
			expect(result.drafts[1].preprompt).toBeUndefined();
		}
	});

	it("forces the branch name through the naming policy prefix", () => {
		const output = JSON.stringify([{ contextName: "New API", branchName: "New API!!" }]);

		const result = parseBulkContextDrafts(output, REQUIRED_PREFIX_POLICY);

		expect(result.ok).toBe(true);
		if (result.ok) expect(result.drafts[0].branchName).toBe("feature/new-api");
	});

	it("seeds the branch name from the context name when branchName is missing", () => {
		const output = JSON.stringify([{ contextName: "Clean Up Logs" }]);

		const result = parseBulkContextDrafts(output, NO_PREFIX_POLICY);

		expect(result.ok).toBe(true);
		if (result.ok) expect(result.drafts[0].branchName).toBe("clean-up-logs");
	});

	it("drops rows without a usable context name", () => {
		const output = JSON.stringify([
			{ branchName: "orphan" },
			{ contextName: "Valid", branchName: "valid" },
		]);

		const result = parseBulkContextDrafts(output, NO_PREFIX_POLICY);

		expect(result.ok).toBe(true);
		if (result.ok) {
			expect(result.drafts).toHaveLength(1);
			expect(result.drafts[0].contextName).toBe("Valid");
		}
	});

	it("drops rows whose branch name slugifies to empty", () => {
		const output = JSON.stringify([
			{ contextName: "!!!", branchName: "???" },
			{ contextName: "Keep", branchName: "keep" },
		]);

		const result = parseBulkContextDrafts(output, NO_PREFIX_POLICY);

		expect(result.ok).toBe(true);
		if (result.ok) {
			expect(result.drafts).toHaveLength(1);
			expect(result.drafts[0].contextName).toBe("Keep");
		}
	});

	it("returns no-json-found when there is no array", () => {
		const result = parseBulkContextDrafts("I could not do that.", NO_PREFIX_POLICY);

		expect(result).toEqual({ ok: false, error: "no-json-found" });
	});

	it("returns invalid-json when the array is malformed", () => {
		const result = parseBulkContextDrafts('[{ "contextName": }]', NO_PREFIX_POLICY);

		expect(result).toEqual({ ok: false, error: "invalid-json" });
	});

	it("returns no-valid-rows when every row is invalid", () => {
		const output = JSON.stringify([{ branchName: "x" }, { contextName: "" }]);

		const result = parseBulkContextDrafts(output, NO_PREFIX_POLICY);

		expect(result).toEqual({ ok: false, error: "no-valid-rows" });
	});
});
