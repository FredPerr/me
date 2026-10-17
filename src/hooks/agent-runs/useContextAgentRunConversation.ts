import { useMemo } from "react";
import {
	buildAgentRunConversation,
	type LiveAgentRun,
} from "@/models/agent-runs/agentRunConversation";
import type { KiroConversation } from "@/models/ai-session/KiroConversation";
import { useContextAgentRuns } from "./useContextAgentRuns";

/**
 * The conversation for a single context: its recorded runs read from Rust
 * (surviving reloads) merged with the live run streaming now, if any. The
 * resulting {@link KiroConversation} is parsed into chat parts by the existing
 * display adapters, so Kiro `stream-json` rendering is unchanged.
 */
export function useContextAgentRunConversation(
	contextId: string,
	liveRun: LiveAgentRun | undefined,
): KiroConversation | undefined {
	const { data: pastRuns } = useContextAgentRuns(contextId);

	return useMemo(
		() => buildAgentRunConversation(contextId, pastRuns ?? [], liveRun),
		[contextId, pastRuns, liveRun],
	);
}
