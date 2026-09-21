import { describe, expect, it } from "vitest";
import { fuzzyMatch, isFuzzyMatch } from "./fuzzyMatch";

const NO_MATCH = -1;

describe("fuzzyMatch", () => {
	it("returns a non-negative score when the query is an exact substring", () => {
		const score = fuzzyMatch("feature/login", "login");

		expect(score).toBeGreaterThanOrEqual(0);
	});

	it("matches non-contiguous characters that appear in order", () => {
		const score = fuzzyMatch("feature/my-thing", "ftmy");

		expect(score).toBeGreaterThanOrEqual(0);
	});

	it("returns no match when characters are out of order", () => {
		const score = fuzzyMatch("feature/login", "nigol");

		expect(score).toBe(NO_MATCH);
	});

	it("returns no match when a query character is absent", () => {
		const score = fuzzyMatch("main", "xyz");

		expect(score).toBe(NO_MATCH);
	});

	it("is case-insensitive", () => {
		const score = fuzzyMatch("Feature/Login", "login");

		expect(score).toBeGreaterThanOrEqual(0);
	});

	it("returns a zero score for an empty query", () => {
		const score = fuzzyMatch("anything", "");

		expect(score).toBe(0);
	});

	it("ranks a match at the start higher than a match later in the string", () => {
		const atStart = fuzzyMatch("login-page", "login");
		const later = fuzzyMatch("feature/login", "login");

		expect(atStart).toBeGreaterThan(later);
	});

	it("rewards a match that begins right after a separator", () => {
		const afterSeparator = fuzzyMatch("fix/ui", "ui");
		const midWord = fuzzyMatch("guix", "ui");

		expect(afterSeparator).toBeGreaterThan(midWord);
	});

	it("ranks consecutive matches higher than scattered ones", () => {
		const consecutive = fuzzyMatch("release", "rel");
		const scattered = fuzzyMatch("radioelement", "rel");

		expect(consecutive).toBeGreaterThan(scattered);
	});
});

describe("isFuzzyMatch", () => {
	it("returns true when the query matches", () => {
		expect(isFuzzyMatch("feature/login", "flog")).toBe(true);
	});

	it("returns false when the query does not match", () => {
		expect(isFuzzyMatch("main", "develop")).toBe(false);
	});
});
