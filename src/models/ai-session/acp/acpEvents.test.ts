import { describe, expect, it } from "vitest";
import { extractAgentText, parseAcpLine } from "./acpEvents";

// Lines captured from kiro-cli 2.27 `chat --no-interactive --output-format stream-json`.
const RUN_STARTED = '{"type":"runStarted","data":{"payloadSchema":"acp","acpProtocolVersion":1}}';
const METADATA = '{"type":"metadata","data":{"sessionId":"s1","contextUsagePercentage":4.4}}';
const MESSAGE_CHUNK =
	'{"type":"sessionUpdate","data":{"sessionId":"s1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Hi"}}}}';
const TOOL_CALL =
	'{"type":"sessionUpdate","data":{"update":{"sessionUpdate":"tool_call","toolCallId":"t1","title":"Running: echo hi","kind":"execute","rawInput":{"command":"echo hi"},"_meta":{"kiro":{"toolName":"shell"}}}}}';
const TOOL_CALL_UPDATE =
	'{"type":"sessionUpdate","data":{"update":{"sessionUpdate":"tool_call_update","toolCallId":"t1","status":"completed","content":[{"type":"content","content":{"type":"text","text":"hi\\n"}}]}}}';
const RUN_FINISHED =
	'{"type":"runFinished","data":{"status":"success","stopReason":"end_turn","finalText":"Output: hi"}}';

describe("parseAcpLine", () => {
	it("returns null for non-JSON and for unmodeled events", () => {
		expect(parseAcpLine("not json")).toBeNull();
		expect(parseAcpLine("")).toBeNull();
		expect(parseAcpLine(METADATA)).toBeNull();
		expect(parseAcpLine(RUN_STARTED)).toBeNull();
	});

	it("parses an agent message chunk", () => {
		expect(parseAcpLine(MESSAGE_CHUNK)).toEqual({ kind: "agentMessageChunk", text: "Hi" });
	});

	it("parses a tool call with name and command", () => {
		expect(parseAcpLine(TOOL_CALL)).toEqual({
			kind: "toolCall",
			toolCallId: "t1",
			title: "Running: echo hi",
			toolName: "shell",
			command: "echo hi",
		});
	});

	it("parses a tool call update with status and output text", () => {
		expect(parseAcpLine(TOOL_CALL_UPDATE)).toEqual({
			kind: "toolCallUpdate",
			toolCallId: "t1",
			status: "completed",
			outputText: "hi\n",
		});
	});

	it("parses run finished with final text", () => {
		expect(parseAcpLine(RUN_FINISHED)).toEqual({
			kind: "runFinished",
			status: "success",
			finalText: "Output: hi",
		});
	});
});

describe("extractAgentText", () => {
	it("prefers the authoritative finalText when present", () => {
		const text = extractAgentText([MESSAGE_CHUNK, RUN_FINISHED]);

		expect(text).toBe("Output: hi");
	});

	it("concatenates message chunks when there is no finalText", () => {
		const chunkA = MESSAGE_CHUNK;
		const chunkB = MESSAGE_CHUNK.replace('"text":"Hi"', '"text":" there"');

		const text = extractAgentText([chunkA, chunkB]);

		expect(text).toBe("Hi there");
	});

	it("ignores metadata and non-JSON noise", () => {
		const text = extractAgentText(["noise", METADATA, MESSAGE_CHUNK]);

		expect(text).toBe("Hi");
	});
});
