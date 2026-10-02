import { describe, expect, it } from "vitest";
import { matchesQuery } from "./SlashProvider";

const CANDIDATE = "testing-conventions";

describe("matchesQuery", () => {
	it("matches everything when the query is empty or whitespace", () => {
		expect(matchesQuery(CANDIDATE, "")).toBe(true);
		expect(matchesQuery(CANDIDATE, "   ")).toBe(true);
	});

	it("matches case-insensitively on substrings", () => {
		expect(matchesQuery(CANDIDATE, "CONV")).toBe(true);
		expect(matchesQuery(CANDIDATE, "testing")).toBe(true);
	});

	it("does not match when the substring is absent", () => {
		expect(matchesQuery(CANDIDATE, "deploy")).toBe(false);
	});
});
