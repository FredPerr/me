import { describe, expect, it } from "vitest";
import { slugify } from "./slugify";

describe("slugify", () => {
	it("converts a string to lowercase", () => {
		const input = "HELLO";
		const expected = "hello";

		const result = slugify(input);

		expect(result).toBe(expected);
	});

	it("replaces spaces with a single hyphen", () => {
		const input = "hello world";
		const expected = "hello-world";

		const result = slugify(input);

		expect(result).toBe(expected);
	});

	it("collapses multiple consecutive spaces into a single hyphen", () => {
		const input = "hello   world";
		const expected = "hello-world";

		const result = slugify(input);

		expect(result).toBe(expected);
	});

	it("trims leading and trailing whitespace before slugifying", () => {
		const input = "  hello world  ";
		const expected = "hello-world";

		const result = slugify(input);

		expect(result).toBe(expected);
	});

	it("removes diacritics and normalizes accented characters", () => {
		const input = "Café";
		const expected = "cafe";

		const result = slugify(input);

		expect(result).toBe(expected);
	});

	it("replaces punctuation with a hyphen", () => {
		const input = "file_name (v2).PDF";
		const expected = "file-name-v2-pdf";

		const result = slugify(input);

		expect(result).toBe(expected);
	});

	it("removes leading and trailing hyphens produced by symbols", () => {
		const input = "--hello world--";
		const expected = "hello-world";

		const result = slugify(input);

		expect(result).toBe(expected);
	});

	it("preserves numbers", () => {
		const input = "Item 42";
		const expected = "item-42";

		const result = slugify(input);

		expect(result).toBe(expected);
	});

	it("returns an empty string when input is only whitespace", () => {
		const input = "   ";
		const expected = "";

		const result = slugify(input);

		expect(result).toBe(expected);
	});

	it("returns an empty string when input is empty", () => {
		const input = "";
		const expected = "";

		const result = slugify(input);

		expect(result).toBe(expected);
	});
});
