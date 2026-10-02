/**
 * The CLI-agnostic contract the chat component is built against.
 *
 * The `AgentChat` component never talks to Kiro (or any CLI) directly: it is
 * handed an {@link AgentSession} and only sends prompts, observes messages, and
 * relays permission decisions through it. Each supported CLI provides one
 * implementation (e.g. `KiroAgentSession`), so adding a CLI means adding a
 * session, not changing the chat UI.
 */

import type { AgentMessage, PermissionDecision } from "@/models/agent-chat/AgentMessage";

/** A disposer returned by subscriptions. */
export type Unsubscribe = () => void;

/** High-level state of a session, independent of any CLI's wire protocol. */
export type AgentSessionState = {
	/** The full conversation so far, oldest message first. */
	messages: AgentMessage[];
	/** True while a prompt is being processed. */
	isBusy: boolean;
};

/**
 * Drives one agent conversation. Implementations wrap a specific CLI's
 * transport (Tauri commands/events for Kiro) and expose a uniform surface.
 */
export type AgentSession = {
	/** A stable identifier for the conversation (e.g. a context id). */
	readonly id: string;

	/** Snapshot of the current state. */
	getState(): AgentSessionState;

	/**
	 * Subscribes to state changes. The listener is a plain notifier (no args) —
	 * it fires on every message/part update, and the subscriber reads the new
	 * value via {@link getState}. This shape matches React's
	 * `useSyncExternalStore`. Returns a disposer.
	 */
	subscribe(listener: () => void): Unsubscribe;

	/** Sends a user prompt, starting a new run. Rejects if already busy. */
	send(prompt: string): Promise<void>;

	/** Requests cancellation of the in-flight run, if any. */
	kill(): Promise<void>;

	/**
	 * Answers a pending permission request. A no-op for sessions whose CLI has
	 * no interactive permission protocol (the common case today), so callers can
	 * always wire the UI without feature-detecting.
	 */
	respondToPermission(requestId: string, decision: PermissionDecision): Promise<void>;

	/**
	 * Whether this session can surface and answer permission requests. The chat
	 * uses it only to decide whether to show "waiting for permission" affordances
	 * — permission parts always render when present.
	 */
	readonly supportsPermissions: boolean;
};
