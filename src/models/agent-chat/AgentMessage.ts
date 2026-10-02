/**
 * CLI-agnostic chat model for the reusable agent chat component.
 *
 * A conversation is a list of {@link AgentMessage}s. A user message is a single
 * block of prompt text; an agent message is a sequence of typed
 * {@link MessagePart}s parsed from the CLI's streamed output (plain text today,
 * structured events later). Parts are an open set: a renderer registry maps
 * each part `type` to a component, so new CLIs or new output shapes add a part
 * type and a renderer without touching the chat core.
 */

/** Who authored a message. */
export type MessageRole = "user" | "agent";

/** Lifecycle of an agent message's underlying run. */
export type MessageStatus = "running" | "completed" | "failed" | "killed";

/** Plain prose with no special structure. */
export type TextPart = {
	type: "text";
	text: string;
};

/** A markdown block to render with formatting (headings, lists, emphasis). */
export type MarkdownPart = {
	type: "markdown";
	markdown: string;
};

/** A fenced code block with an optional language hint. */
export type CodeBlockPart = {
	type: "code";
	code: string;
	language?: string;
};

/**
 * A tool/command the agent invoked, surfaced structurally so the UI can show
 * what the CLI is doing. Populated from structured events when the backend
 * provides them; absent in pure plain-text mode.
 */
export type ToolCallPart = {
	type: "toolCall";
	/** Tool or command name, e.g. "fs_write" or "git status". */
	name: string;
	/** Human-readable summary of the arguments, when available. */
	summary?: string;
	status: "running" | "completed" | "failed";
};

/** A line of raw terminal output tagged by stream, used as a fallback. */
export type StreamLinePart = {
	type: "streamLine";
	stream: "stdout" | "stderr";
	text: string;
};

/**
 * A request from the CLI for permission to run a command or use a tool. The
 * chat renders allow/deny controls that call back into the session. The backend
 * must support an interactive protocol for this to appear; the type and UI are
 * defined now so the frontend is ready.
 */
export type PermissionRequestPart = {
	type: "permissionRequest";
	/** Correlates the user's decision back to the pending request. */
	requestId: string;
	/** What the agent wants to do, e.g. run a shell command. */
	title: string;
	/** Optional detail: the exact command, path, or tool arguments. */
	detail?: string;
	/** Set once the user (or a prior auto-rule) has resolved it. */
	resolution?: PermissionDecision;
};

/** The ways a permission request can be answered. */
export type PermissionDecision = "allowOnce" | "allowAlways" | "deny";

/** An error surfaced within an agent message. */
export type ErrorPart = {
	type: "error";
	message: string;
};

/**
 * The open set of renderable parts. Adding a part type here and registering a
 * renderer is the extension point for richer output.
 */
export type MessagePart =
	| TextPart
	| MarkdownPart
	| CodeBlockPart
	| ToolCallPart
	| StreamLinePart
	| PermissionRequestPart
	| ErrorPart;

/** A single message in the conversation. */
export type AgentMessage = {
	id: string;
	role: MessageRole;
	/** Present on agent messages; user messages use a single text part. */
	parts: MessagePart[];
	status: MessageStatus;
	createdAt: number;
};

/** True when the message's run is still in flight. */
export function isMessageRunning(message: AgentMessage): boolean {
	return message.status === "running";
}

/** The first unresolved permission request across a message's parts, if any. */
export function pendingPermission(message: AgentMessage): PermissionRequestPart | undefined {
	return message.parts.find(
		(part): part is PermissionRequestPart =>
			part.type === "permissionRequest" && part.resolution === undefined,
	);
}
