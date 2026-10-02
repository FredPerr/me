import { describe, expect, it } from "vitest";
import { parseAgentOutput } from "./parseAgentOutput";

describe("parseAgentOutput", () => {
	it("returns no parts for empty or whitespace-only output", () => {
		expect(parseAgentOutput("")).toEqual([]);
		expect(parseAgentOutput("   \n  ")).toEqual([]);
	});

	it("returns a single markdown part for plain prose", () => {
		const result = parseAgentOutput("Here is a plan for the change.");

		expect(result).toEqual([{ type: "markdown", markdown: "Here is a plan for the change." }]);
	});

	it("extracts a fenced code block with its language", () => {
		const output = "Run this:\n```ts\nconst x = 1;\n```\nDone.";

		const result = parseAgentOutput(output);

		expect(result).toEqual([
			{ type: "markdown", markdown: "Run this:" },
			{ type: "code", code: "const x = 1;", language: "ts" },
			{ type: "markdown", markdown: "Done." },
		]);
	});

	it("omits the language when the fence has none", () => {
		const output = "```\nplain code\n```";

		const result = parseAgentOutput(output);

		expect(result).toEqual([{ type: "code", code: "plain code" }]);
	});

	it("handles multiple code blocks interleaved with prose", () => {
		const output = "First:\n```js\na()\n```\nThen:\n```js\nb()\n```";

		const result = parseAgentOutput(output);

		expect(result).toEqual([
			{ type: "markdown", markdown: "First:" },
			{ type: "code", code: "a()", language: "js" },
			{ type: "markdown", markdown: "Then:" },
			{ type: "code", code: "b()", language: "js" },
		]);
	});

	it("drops empty prose between adjacent fences", () => {
		const output = "```\none\n```\n\n```\ntwo\n```";

		const result = parseAgentOutput(output);

		expect(result).toEqual([
			{ type: "code", code: "one" },
			{ type: "code", code: "two" },
		]);
	});
});
