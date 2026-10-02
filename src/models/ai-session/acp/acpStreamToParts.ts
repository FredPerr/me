/**
 * Converts a Kiro `stream-json` line stream into renderable chat parts.
 *
 * Agent message chunks are concatenated and parsed into markdown/code parts;
 * tool calls become {@link ToolCallPart}s keyed by their tool-call id and
 * updated in place as `tool_call_update` events arrive. Parts are emitted in
 * the order their triggering events first appear, so a tool call rendered
 * between two spans of prose keeps its position. Lines that are not modeled ACP
 * events (plain text, or an unexpected shape) are collected as stderr-less raw
 * stream lines so nothing is silently dropped.
 */

import type { MessagePart, ToolCallPart } from "@/models/agent-chat/AgentMessage";
import { parseAgentOutput } from "@/models/agent-chat/parseAgentOutput";
import { parseAcpLine } from "@/models/ai-session/acp/acpEvents";

/** True when at least one line parses as a modeled ACP event. */
export function looksLikeAcpStream(lines: string[]): boolean {
	return lines.some((line) => parseAcpLine(line) !== null);
}

type Segment =
	| { kind: "text"; text: string }
	| { kind: "tool"; toolCallId: string; tool: ToolCallPart };

function toolTitle(title: string, command?: string): string {
	if (title.trim() !== "") return title;
	return command ?? "tool";
}

/**
 * Parses the full line stream into ordered parts. Consecutive text chunks are
 * merged into one segment so a single markdown block is produced per prose run
 * (rather than one per streamed token).
 */
export function acpStreamToParts(lines: string[]): MessagePart[] {
	const segments: Segment[] = [];
	const toolsById = new Map<string, ToolCallPart>();

	function appendText(text: string) {
		const last = segments[segments.length - 1];
		if (last && last.kind === "text") {
			last.text += text;
		} else {
			segments.push({ kind: "text", text });
		}
	}

	for (const line of lines) {
		const event = parseAcpLine(line);
		if (!event) continue;

		if (event.kind === "agentMessageChunk") {
			appendText(event.text);
			continue;
		}

		if (event.kind === "toolCall") {
			const tool: ToolCallPart = {
				type: "toolCall",
				name: event.toolName ?? "tool",
				summary: toolTitle(event.title, event.command),
				status: "running",
			};
			toolsById.set(event.toolCallId, tool);
			segments.push({ kind: "tool", toolCallId: event.toolCallId, tool });
			continue;
		}

		if (event.kind === "toolCallUpdate") {
			const existing = toolsById.get(event.toolCallId);
			if (existing && event.status) existing.status = event.status;
		}
	}

	return segments.flatMap((segment) =>
		segment.kind === "tool" ? [segment.tool] : parseAgentOutput(segment.text),
	);
}
