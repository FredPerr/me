/**
 * Frontend types for the interactive AI chat interface.
 *
 * These extend the fire-and-forget session model in `models/ai-session` with
 * an interactive, turn-based conversation plus the permission requests an agent
 * raises while it works (e.g. a tool asking for write access).
 *
 * The wire types (`ChatMessageEvent`, `PermissionRequestEvent`,
 * `PermissionResolvedEvent`, `ChatInput`, `PermissionDecisionInput`) mirror the
 * Rust structs that back them, serialized camelCase — kept in sync by hand,
 * matching the repo convention (see `models/ai-session/AiSession.ts`).
 */

/** Who authored a chat message. */
export type ChatRole = "user" | "assistant" | "system";

/**
 * A single turn in the conversation. `pending` marks an assistant turn that is
 * still streaming so the UI can show a typing indicator and append deltas.
 */
export type ChatMessage = {
	id: string;
	sessionId: string;
	role: ChatRole;
	text: string;
	createdAt: number;
	pending?: boolean;
};

/** A streamed chat message chunk from the agent; mirrors Rust `ChatMessageEvent`. */
export type ChatMessageEvent = {
	sessionId: string;
	messageId: string;
	role: ChatRole;
	/** Incremental text to append to the message with `messageId`. */
	delta: string;
	/** True on the final chunk of a message, so the UI can clear `pending`. */
	done: boolean;
};

/** How a resolved permission request was decided. */
export type PermissionDecision = "allow" | "allow_always" | "deny";

/**
 * A request from the agent for the user to approve or deny an action (the
 * "permission request" that previously could only be answered in a modal).
 * Mirrors Rust `PermissionRequestEvent`.
 */
export type PermissionRequest = {
	id: string;
	sessionId: string;
	/** The tool or capability requesting access, e.g. `"fs_write"`. */
	tool: string;
	/** Human-readable summary of what the agent wants to do. */
	title: string;
	/** Optional longer detail, e.g. the command or file path involved. */
	detail?: string;
	createdAt: number;
};

/** Notification that a permission request was resolved; mirrors Rust `PermissionResolvedEvent`. */
export type PermissionResolvedEvent = {
	sessionId: string;
	requestId: string;
	decision: PermissionDecision;
};

/** Payload for sending a chat turn to a running session; mirrors Rust `ChatInput`. */
export type ChatInput = {
	sessionId: string;
	text: string;
};

/** Payload for answering a permission request; mirrors Rust `PermissionDecisionInput`. */
export type PermissionDecisionInput = {
	sessionId: string;
	requestId: string;
	decision: PermissionDecision;
};
