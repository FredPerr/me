/**
 * Maps the Kiro-specific conversation transcript into the CLI-agnostic
 * {@link AgentMessage} model the reusable chat renders.
 *
 * Each {@link KiroTurn} becomes two messages: the user's prompt, then the
 * agent's response. The agent's streamed stdout lines are joined and parsed
 * into rich parts (markdown/code); stderr lines are kept as raw stream-line
 * parts so errors stay visible verbatim. A failed/killed turn appends an error
 * part when it carries an error message.
 */

import type { AgentMessage, MessagePart, MessageStatus } from "@/models/agent-chat/AgentMessage";
import { parseAgentOutput } from "@/models/agent-chat/parseAgentOutput";
import type { SessionStatus } from "@/models/ai-session/AiSession";
import { acpStreamToParts, looksLikeAcpStream } from "@/models/ai-session/acp/acpStreamToParts";
import type { KiroConversation, KiroTurn } from "@/models/ai-session/KiroConversation";

function toMessageStatus(status: SessionStatus): MessageStatus {
	return status;
}

/**
 * Builds the agent-response parts for a turn. Kiro runs with
 * `--output-format stream-json`, so stdout is normally a stream of ACP events
 * parsed into rich parts; if the lines are not an ACP stream (older CLI, or an
 * unexpected mode) we fall back to treating stdout as plain markdown text.
 */
function agentParts(turn: KiroTurn): MessagePart[] {
	const stdoutLines = turn.lines
		.filter((line) => line.stream === "stdout")
		.map((line) => line.text);

	const parts: MessagePart[] = looksLikeAcpStream(stdoutLines)
		? acpStreamToParts(stdoutLines)
		: parseAgentOutput(stdoutLines.join("\n"));

	for (const line of turn.lines) {
		if (line.stream === "stderr") {
			parts.push({ type: "streamLine", stream: "stderr", text: line.text });
		}
	}

	if (turn.error) {
		parts.push({ type: "error", message: turn.error });
	}

	return parts;
}

/** Converts a Kiro conversation (or none) into agent messages, in order. */
export function kiroConversationToMessages(
	conversation: KiroConversation | undefined,
): AgentMessage[] {
	if (!conversation) return [];

	return conversation.turns.flatMap((turn) => {
		const userMessage: AgentMessage = {
			id: `${turn.sessionId}-user`,
			role: "user",
			parts: [{ type: "text", text: turn.prompt }],
			status: "completed",
			createdAt: turn.startedAt,
		};
		const agentMessage: AgentMessage = {
			id: `${turn.sessionId}-agent`,
			role: "agent",
			parts: agentParts(turn),
			status: toMessageStatus(turn.status),
			createdAt: turn.startedAt,
		};
		return [userMessage, agentMessage];
	});
}
