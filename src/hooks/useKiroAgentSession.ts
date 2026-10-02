import { useEffect, useMemo, useRef } from "react";
import type { PermissionDecision } from "@/models/agent-chat/AgentMessage";
import type { AgentSession, AgentSessionState } from "@/models/agent-chat/AgentSession";
import type { KiroConversation } from "@/models/ai-session/KiroConversation";
import { kiroConversationToMessages } from "@/models/ai-session/kiroConversationToMessages";

type KiroAgentSessionParams = {
	/** Stable id for the conversation (the context id). */
	id: string;
	conversation: KiroConversation | undefined;
	isRunning: boolean;
	onSend: (prompt: string) => Promise<void>;
	onKill: () => Promise<void>;
};

/**
 * Adapts the existing Kiro conversation state (fed in as props from
 * `useContextKiroSessions`) into the CLI-agnostic {@link AgentSession} the
 * reusable chat consumes. The session object is stable for the component's
 * lifetime; prop changes are mirrored into a ref and broadcast to subscribers,
 * so `AgentChat`'s `useSyncExternalStore` re-renders on every streamed update.
 *
 * Kiro runs headlessly with tools trusted (`--trust-all-tools`), governed by a
 * workspace `permissions.yaml` deny floor — it never pauses to ask, so it has
 * no interactive permission protocol: `supportsPermissions` is false and
 * `respondToPermission` is a no-op. The permission part/UI remain wired for a
 * future CLI that does prompt interactively.
 */
export function useKiroAgentSession({
	id,
	conversation,
	isRunning,
	onSend,
	onKill,
}: KiroAgentSessionParams): AgentSession {
	// Latest derived state, read by getState without re-creating the session.
	const stateRef = useRef<AgentSessionState>({ messages: [], isBusy: false });
	// Latest callbacks, so send/kill always call the current handlers.
	const callbacksRef = useRef({ onSend, onKill });
	callbacksRef.current = { onSend, onKill };

	const listeners = useRef(new Set<() => void>());

	// Recompute derived state when inputs change and notify subscribers.
	useEffect(() => {
		stateRef.current = {
			messages: kiroConversationToMessages(conversation),
			isBusy: isRunning,
		};
		for (const listener of listeners.current) listener();
	}, [conversation, isRunning]);

	return useMemo<AgentSession>(
		() => ({
			id,
			supportsPermissions: false,
			getState: () => stateRef.current,
			subscribe: (listener) => {
				listeners.current.add(listener);
				return () => listeners.current.delete(listener);
			},
			send: (prompt) => callbacksRef.current.onSend(prompt),
			kill: () => callbacksRef.current.onKill(),
			respondToPermission: async (_requestId: string, _decision: PermissionDecision) => {
				// No interactive permission protocol for headless Kiro yet.
			},
		}),
		[id],
	);
}
