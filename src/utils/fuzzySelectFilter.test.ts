import type { ComboboxItem } from "@mantine/core";
import { describe, expect, it } from "vitest";
import { fuzzySelectFilter } from "./fuzzySelectFilter";

function buildOptions(...labels: string[]): ComboboxItem[] {
	return labels.map((label) => ({ value: label, label }));
}

describe("fuzzySelectFilter", () => {
	const LIMIT = 100;

	it("returns all options when the search is empty", () => {
		const options = buildOptions("main", "develop", "feature/login");

		const result = fuzzySelectFilter({ options, search: "", limit: LIMIT });

		expect(result).toHaveLength(3);
	});

	it("keeps only options that fuzzy-match the search", () => {
		const options = buildOptions("main", "develop", "feature/login");

		const result = fuzzySelectFilter({ options, search: "login", limit: LIMIT });

		expect(result.map((o) => ("label" in o ? o.label : ""))).toEqual(["feature/login"]);
	});

	it("orders more relevant matches first", () => {
		const options = buildOptions("feature/login", "login-page");

		const result = fuzzySelectFilter({ options, search: "login", limit: LIMIT });

		expect("label" in result[0] ? result[0].label : "").toBe("login-page");
	});
});
