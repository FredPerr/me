const AGENT_RUNS_QUERY_ROOT = "agentRuns";

/** Query key for the list of past agent runs recorded for a context. */
export function contextAgentRunsQueryKey(contextId: string) {
	return [AGENT_RUNS_QUERY_ROOT, "context", contextId] as const;
}
