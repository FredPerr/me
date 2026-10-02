/**
 * Typed view over Kiro CLI's `--output-format stream-json` output.
 *
 * In that mode the CLI prints one self-describing JSON object per line (ACP
 * events) on stdout. We only model the fields the chat needs; everything else
 * (metadata credits, context usage) is ignored. Lines that are not valid JSON,
 * or events we do not model, parse to `null` so callers can fall back to raw
 * rendering without throwing.
 *
 * Event shapes observed from kiro-cli 2.27 (`--output-format stream-json`):
 *   {"type":"runStarted","data":{...}}
 *   {"type":"sessionUpdate","data":{"update":{"sessionUpdate":"agent_message_chunk",
 *       "content":{"type":"text","text":"..."}}}}
 *   {"type":"sessionUpdate","data":{"update":{"sessionUpdate":"tool_call",
 *       "toolCallId":"...","title":"...","kind":"execute",
 *       "rawInput":{"command":"..."},"_meta":{"kiro":{"toolName":"shell"}}}}}
 *   {"type":"sessionUpdate","data":{"update":{"sessionUpdate":"tool_call_update",
 *       "toolCallId":"...","status":"completed","content":[...],"rawOutput":{...}}}}
 *   {"type":"runFinished","data":{"status":"success","finalText":"..."}}
 */

/** A streamed chunk of the agent's natural-language answer. */
export type AgentMessageChunkEvent = {
	kind: "agentMessageChunk";
	text: string;
};

/** The agent started invoking a tool/command. */
export type ToolCallEvent = {
	kind: "toolCall";
	toolCallId: string;
	title: string;
	/** The tool name from `_meta.kiro.toolName`, e.g. "shell", "fs_write". */
	toolName?: string;
	/** The command string for shell tools, from `rawInput.command`. */
	command?: string;
};

/** Progress or completion of a previously announced tool call. */
export type ToolCallUpdateEvent = {
	kind: "toolCallUpdate";
	toolCallId: string;
	status?: "running" | "completed" | "failed";
	/** Any textual output chunks carried by the update. */
	outputText?: string;
};

/** The run finished; carries the final concatenated answer. */
export type RunFinishedEvent = {
	kind: "runFinished";
	status: string;
	finalText?: string;
};

/** The modeled subset of ACP events. */
export type AcpEvent =
	| AgentMessageChunkEvent
	| ToolCallEvent
	| ToolCallUpdateEvent
	| RunFinishedEvent;

function asRecord(value: unknown): Record<string, unknown> | null {
	return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : null;
}

function asString(value: unknown): string | undefined {
	return typeof value === "string" ? value : undefined;
}

/** Extracts the text from an ACP content object `{type:"text", text:"..."}`. */
function contentText(content: unknown): string | undefined {
	const record = asRecord(content);
	if (!record) return undefined;
	return asString(record.text);
}

/** Joins the text of a `tool_call_update.content` array of content wrappers. */
function toolUpdateText(content: unknown): string | undefined {
	if (!Array.isArray(content)) return undefined;
	const texts = content
		.map((item) => {
			const wrapper = asRecord(item);
			// Shape: { type: "content", content: { type: "text", text } }
			return wrapper ? contentText(wrapper.content) : undefined;
		})
		.filter((text): text is string => text !== undefined);
	return texts.length > 0 ? texts.join("") : undefined;
}

function parseStatus(value: unknown): ToolCallUpdateEvent["status"] {
	if (value === "running" || value === "completed" || value === "failed") return value;
	return undefined;
}

/**
 * Parses one line of stream-json into a modeled {@link AcpEvent}, or null when
 * the line is not JSON or is an event type we do not render.
 */
export function parseAcpLine(line: string): AcpEvent | null {
	const trimmed = line.trim();
	if (trimmed === "") return null;

	let parsed: unknown;
	try {
		parsed = JSON.parse(trimmed);
	} catch {
		return null;
	}

	const envelope = asRecord(parsed);
	if (!envelope) return null;
	const type = asString(envelope.type);
	const data = asRecord(envelope.data);
	if (!data) return null;

	if (type === "runFinished") {
		return {
			kind: "runFinished",
			status: asString(data.status) ?? "unknown",
			finalText: asString(data.finalText),
		};
	}

	if (type !== "sessionUpdate") return null;

	const update = asRecord(data.update);
	if (!update) return null;
	const sessionUpdate = asString(update.sessionUpdate);

	if (sessionUpdate === "agent_message_chunk") {
		const text = contentText(update.content);
		return text === undefined ? null : { kind: "agentMessageChunk", text };
	}

	if (sessionUpdate === "tool_call") {
		const rawInput = asRecord(update.rawInput);
		const meta = asRecord(update._meta);
		const kiroMeta = meta ? asRecord(meta.kiro) : null;
		return {
			kind: "toolCall",
			toolCallId: asString(update.toolCallId) ?? "",
			title: asString(update.title) ?? "",
			toolName: kiroMeta ? asString(kiroMeta.toolName) : undefined,
			command: rawInput ? asString(rawInput.command) : undefined,
		};
	}

	if (sessionUpdate === "tool_call_update") {
		return {
			kind: "toolCallUpdate",
			toolCallId: asString(update.toolCallId) ?? "",
			status: parseStatus(update.status),
			outputText: toolUpdateText(update.content),
		};
	}

	return null;
}

/**
 * Reconstructs the agent's natural-language answer from a stream of lines by
 * concatenating `agent_message_chunk` text, preferring `runFinished.finalText`
 * when present (it is the authoritative, de-chunked answer). Used where only
 * the final text matters, such as parsing a structured reply.
 */
export function extractAgentText(lines: string[]): string {
	let chunks = "";
	for (const line of lines) {
		const event = parseAcpLine(line);
		if (!event) continue;
		if (event.kind === "runFinished" && event.finalText) return event.finalText;
		if (event.kind === "agentMessageChunk") chunks += event.text;
	}
	return chunks;
}
