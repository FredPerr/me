import { useQuery } from "@tanstack/react-query";
import { type AgentRunView, commands } from "@/bindings";
import { contextAgentRunsQueryKey } from "./agentRunQueryKeys";

/**
 * Reads a context's past agent runs from Rust, so transcripts are shown after a
 * reload. Rust keeps the recent runs in `me.redb`; this is the only source of a
 * run's history on mount.
 */
export function useContextAgentRuns(contextId: string) {
	return useQuery<AgentRunView[]>({
		queryKey: contextAgentRunsQueryKey(contextId),
		queryFn: () => commands.workspaceListAgentRuns(contextId),
	});
}
