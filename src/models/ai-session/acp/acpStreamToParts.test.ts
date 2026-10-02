import { describe, expect, it } from "vitest";
import { acpStreamToParts, looksLikeAcpStream } from "./acpStreamToParts";

const MESSAGE_CHUNK =
	'{"type":"sessionUpdate","data":{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Done. "}}}}';
const MESSAGE_CHUNK_2 =
	'{"type":"sessionUpdate","data":{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"All good."}}}}';
const TOOL_CALL =
	'{"type":"sessionUpdate","data":{"update":{"sessionUpdate":"tool_call","toolCallId":"t1","title":"Running: echo hi","kind":"execute","rawInput":{"command":"echo hi"},"_meta":{"kiro":{"toolName":"shell"}}}}}';
const TOOL_DONE =
	'{"type":"sessionUpdate","data":{"update":{"sessionUpdate":"tool_call_update","toolCallId":"t1","status":"completed"}}}';

describe("looksLikeAcpStream", () => {
	it("is true when any line is a modeled event", () => {
		expect(looksLikeAcpStream(["noise", MESSAGE_CHUNK])).toBe(true);
	});

	it("is false for plain text output", () => {
		expect(looksLikeAcpStream(["just", "plain", "lines"])).toBe(false);
	});
});

describe("acpStreamToParts", () => {
	it("merges consecutive message chunks into one markdown part", () => {
		const parts = acpStreamToParts([MESSAGE_CHUNK, MESSAGE_CHUNK_2]);

		expect(parts).toEqual([{ type: "markdown", markdown: "Done. All good." }]);
	});

	it("renders a tool call as a running tool part, updated to completed", () => {
		const parts = acpStreamToParts([TOOL_CALL, TOOL_DONE]);

		expect(parts).toEqual([
			{ type: "toolCall", name: "shell", summary: "Running: echo hi", status: "completed" },
		]);
	});

	it("keeps order: prose, then tool, then trailing prose", () => {
		const parts = acpStreamToParts([MESSAGE_CHUNK, TOOL_CALL, TOOL_DONE, MESSAGE_CHUNK_2]);

		expect(parts).toEqual([
			{ type: "markdown", markdown: "Done." },
			{ type: "toolCall", name: "shell", summary: "Running: echo hi", status: "completed" },
			{ type: "markdown", markdown: "All good." },
		]);
	});
});
