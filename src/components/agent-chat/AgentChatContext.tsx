import { createContext, useContext } from "react";
import type { PermissionDecision } from "@/models/agent-chat/AgentMessage";

/**
 * Actions a rendered part may invoke, provided by the chat shell. Kept in
 * context so individual part renderers stay decoupled from the session: a part
 * calls `respondToPermission` without knowing which CLI (or transport) backs
 * the conversation.
 */
export type AgentChatActions = {
	respondToPermission: (requestId: string, decision: PermissionDecision) => void;
	/** Whether the active session can actually act on a permission decision. */
	supportsPermissions: boolean;
};

const AgentChatActionsContext = createContext<AgentChatActions | null>(null);

export const AgentChatActionsProvider = AgentChatActionsContext.Provider;

/** Reads the chat actions. Throws if used outside the chat shell. */
export function useAgentChatActions(): AgentChatActions {
	const actions = useContext(AgentChatActionsContext);
	if (actions === null) {
		throw new Error("useAgentChatActions must be used within an AgentChat");
	}
	return actions;
}
