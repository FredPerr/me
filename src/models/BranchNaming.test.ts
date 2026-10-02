import { describe, expect, it } from "vitest";
import { BranchNamingPolicy, DEFAULT_BRANCH_PREFIXES, parseBranchPrefix } from "./BranchNaming";

const TASK_NAME = "[WAFR] Do something here";

describe("parseBranchPrefix", () => {
	it.each([
		["feature/", "feature"],
		[" chore// ", "chore"],
		["team/feature", "team/feature"],
		["Feature", "Feature"],
	])("normalizes %j to %j", (rawPrefix, expectedPrefix) => {
		expect(parseBranchPrefix(rawPrefix)).toEqual({ valid: true, prefix: expectedPrefix });
	});

	it.each(["", "  ", "/"])("rejects %j as empty", (rawPrefix) => {
		expect(parseBranchPrefix(rawPrefix)).toEqual({ valid: false, reason: "empty" });
	});

	it.each([
		"my prefix",
		"a..b",
		"a~b",
		"a^b",
		"a:b",
		"a?b",
		"a*b",
		"a[b",
		"a\\b",
		"@",
		"a@{b",
		".hidden",
		"x.lock",
		"x.",
		"a//b",
	])("rejects %j as invalid", (rawPrefix) => {
		expect(parseBranchPrefix(rawPrefix)).toEqual({ valid: false, reason: "invalid" });
	});
});

describe("BranchNamingPolicy", () => {
	describe("composeBranchName", () => {
		it("returns the slug when no prefix is chosen", () => {
			const policy = BranchNamingPolicy.default();
			expect(policy.composeBranchName(TASK_NAME, null)).toBe("wafr-do-something-here");
		});

		it.each([
			["feature", "feature/wafr-do-something-here"],
			["fix", "fix/wafr-do-something-here"],
		])("prepends the %s prefix", (prefix, expectedBranchName) => {
			const policy = BranchNamingPolicy.default();
			expect(policy.composeBranchName(TASK_NAME, prefix)).toBe(expectedBranchName);
		});

		it("returns an empty string when the task name has no alphanumeric characters", () => {
			const policy = BranchNamingPolicy.default();
			expect(policy.composeBranchName("  !!!  ", "feature")).toBe("");
		});

		it("throws when no prefix is chosen but a prefix is required", () => {
			const policy = BranchNamingPolicy.create(["feature"], false);
			expect(() => policy.composeBranchName(TASK_NAME, null)).toThrow();
		});

		it("throws when the prefix is not configured", () => {
			const policy = BranchNamingPolicy.default();
			expect(() => policy.composeBranchName(TASK_NAME, "hotfix")).toThrow();
		});
	});

	describe("create", () => {
		it("drops invalid and empty prefixes, normalizes and dedupes while keeping order", () => {
			const policy = BranchNamingPolicy.create(
				["fix/", "", "my prefix", "feature", "feature/", " chore "],
				true,
			);
			expect(policy.prefixes).toEqual(["fix", "feature", "chore"]);
		});
	});

	describe("allowsNoPrefix and defaultPrefix", () => {
		it("defaults to no prefix when no prefix is allowed", () => {
			const policy = BranchNamingPolicy.create(["feature"], true);
			expect(policy.allowsNoPrefix).toBe(true);
			expect(policy.defaultPrefix).toBeNull();
		});

		it("defaults to the first prefix when a prefix is required", () => {
			const policy = BranchNamingPolicy.create(["fix", "feature"], false);
			expect(policy.allowsNoPrefix).toBe(false);
			expect(policy.defaultPrefix).toBe("fix");
		});

		it("allows no prefix when the prefix list is empty even if disallowed", () => {
			const policy = BranchNamingPolicy.create([], false);
			expect(policy.allowsNoPrefix).toBe(true);
			expect(policy.defaultPrefix).toBeNull();
		});
	});

	describe("fromJSON", () => {
		it.each([undefined, {}])("uses defaults for %j", (data) => {
			const policy = BranchNamingPolicy.fromJSON(data);
			expect(policy.prefixes).toEqual(DEFAULT_BRANCH_PREFIXES);
			expect(policy.allowsNoPrefix).toBe(true);
		});

		it("keeps default prefixes when only allowNoPrefix is set", () => {
			const policy = BranchNamingPolicy.fromJSON({ allowNoPrefix: false });
			expect(policy.prefixes).toEqual(DEFAULT_BRANCH_PREFIXES);
			expect(policy.allowsNoPrefix).toBe(false);
		});

		it("keeps custom prefixes and allows no prefix by default", () => {
			const policy = BranchNamingPolicy.fromJSON({ prefixes: ["ops"] });
			expect(policy.prefixes).toEqual(["ops"]);
			expect(policy.allowsNoPrefix).toBe(true);
		});

		it("keeps an empty prefix list", () => {
			expect(BranchNamingPolicy.fromJSON({ prefixes: [] }).prefixes).toEqual([]);
		});

		it("round-trips through toJSON, preserving the raw allowNoPrefix setting", () => {
			const data = { prefixes: [], allowNoPrefix: false };
			expect(BranchNamingPolicy.fromJSON(data).toJSON()).toEqual(data);
		});
	});
});
