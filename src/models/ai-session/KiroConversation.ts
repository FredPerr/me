/**
 * Conversation transcript for a context's Kiro runs.
 *
 * A context accumulates one or more runs over time. Each run is a `KiroTurn`:
 * the user's prompt plus the agent's streamed output lines and the run's final
 * status. The transcript is ephemeral UI state (not persisted), kept in memory
 * for the lifetime of the kanban view.
 */

import type { OutputStream, SessionStatus } from "@/models/ai-session/AiSession";

/** A single streamed line of agent output within a turn. */
export type ConversationLine = {
	id: number;
	stream: OutputStream;
	text: string;
};

/** One run: the prompt that started it and the agent's response so far. */
export type KiroTurn = {
	sessionId: string;
	prompt: string;
	status: SessionStatus;
	exitCode: number | null;
	error: string | null;
	startedAt: number;
	lines: ConversationLine[];
};

/** The full transcript for a context, newest-first turns kept in order sent. */
export type KiroConversation = {
	contextId: string;
	turns: KiroTurn[];
};

/** True when the context has an actively running turn. */
export function hasRunningTurn(conversation: KiroConversation | undefined): boolean {
	return Boolean(conversation?.turns.some((turn) => turn.status === "running"));
}
