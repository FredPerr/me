import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { type AgentRunChanged, events } from "@/bindings";
import type { LiveAgentRun } from "@/models/agent-runs/agentRunConversation";
import { contextAgentRunsQueryKey } from "./agentRunQueryKeys";

/** The live run per context, keyed by context id, built from the event stream. */
type LiveRunsByContext = Record<string, LiveAgentRun>;

export type AgentRunStream = {
	/** The live run for a context, if one is streaming right now. */
	liveRunFor: (contextId: string) => LiveAgentRun | undefined;
	/** Whether the context's latest run is in flight (started, not yet ended). */
	isRunning: (contextId: string) => boolean;
	/**
	 * Remembers the prompt a run is about to start with, so the live transcript
	 * can show it: the `started` event carries only ids, not the prompt.
	 */
	notePendingPrompt: (contextId: string, prompt: string) => void;
};

function applyUpdate(
	current: LiveRunsByContext,
	event: AgentRunChanged,
	pendingPrompts: Record<string, string>,
): LiveRunsByContext {
	const { contextId, runId, update } = event;

	switch (update.kind) {
		case "started":
			return {
				...current,
				[contextId]: {
					runId,
					contextId,
					prompt: pendingPrompts[contextId] ?? "",
					lines: [],
					state: "running",
					startedAt: Date.now(),
				},
			};
		case "line": {
			const run = current[contextId];
			if (!run || run.runId !== runId) return current;
			return {
				...current,
				[contextId]: { ...run, lines: [...run.lines, update.text] },
			};
		}
		case "ended": {
			const run = current[contextId];
			if (!run || run.runId !== runId) return current;
			return {
				...current,
				[contextId]: { ...run, state: update.finalState },
			};
		}
	}
}

/**
 * Subscribes once to the `agent-run` event for the whole board and keeps the
 * live run per context in memory. Appends `line` updates to the running run and
 * reflects `started`/`ended`; on `ended` it refetches that context's recorded
 * runs so the persisted transcript replaces the live one. Rust owns the run; the
 * frontend only mirrors what it reports.
 */
export function useAgentRunStream(): AgentRunStream {
	const queryClient = useQueryClient();
	const [liveRuns, setLiveRuns] = useState<LiveRunsByContext>({});
	const pendingPrompts = useRef<Record<string, string>>({});

	useEffect(() => {
		let disposed = false;
		let stopListening: (() => void) | undefined;

		events.agentRunChanged
			.listen((event) => {
				const changed = event.payload;
				setLiveRuns((current) => applyUpdate(current, changed, pendingPrompts.current));
				if (changed.update.kind === "ended") {
					queryClient.invalidateQueries({
						queryKey: contextAgentRunsQueryKey(changed.contextId),
					});
				}
			})
			.then((unlisten) => {
				if (disposed) unlisten();
				else stopListening = unlisten;
			});

		return () => {
			disposed = true;
			stopListening?.();
		};
	}, [queryClient]);

	const liveRunFor = useCallback((contextId: string) => liveRuns[contextId], [liveRuns]);

	const isRunning = useCallback(
		(contextId: string) => liveRuns[contextId]?.state === "running",
		[liveRuns],
	);

	const notePendingPrompt = useCallback((contextId: string, prompt: string) => {
		pendingPrompts.current[contextId] = prompt;
	}, []);

	return { liveRunFor, isRunning, notePendingPrompt };
}
