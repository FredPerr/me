import { describe, expect, it } from "vitest";
import { detectSlashQuery } from "./ChatInput";

describe("detectSlashQuery", () => {
	it("detects a slash token at the start of the input", () => {
		const value = "/steer";

		const result = detectSlashQuery(value, value.length);

		expect(result).toEqual({ start: 0, text: "steer" });
	});

	it("detects a slash token after whitespace", () => {
		const value = "look at /file";

		const result = detectSlashQuery(value, value.length);

		expect(result).toEqual({ start: 8, text: "file" });
	});

	it("detects an empty query right after the slash", () => {
		const value = "/";

		const result = detectSlashQuery(value, 1);

		expect(result).toEqual({ start: 0, text: "" });
	});

	it("returns null when the caret is after a completed token with a space", () => {
		const value = "/file done";

		const result = detectSlashQuery(value, value.length);

		expect(result).toBeNull();
	});

	it("returns null for a slash in the middle of a word (e.g. a path)", () => {
		const value = "src/models";

		const result = detectSlashQuery(value, value.length);

		expect(result).toBeNull();
	});
});
