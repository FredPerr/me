import { useCallback } from "react";
import type { ProjectView } from "@/bindings";
import type { LiveAgentRun } from "@/models/agent-runs/agentRunConversation";
import { useStartAgentRun, useStopAgentRun } from "./useAgentRunMutations";
import { useAgentRunStream } from "./useAgentRunStream";

type RunAgentParams = {
	contextId: string;
	prompt: string;
};

export type ContextAgentRunSessions = {
	/** Starts an agent run on a context through the Rust use case. */
	runAgent: (params: RunAgentParams) => Promise<void>;
	/** Stops the running agent for a context. */
	stopAgentRun: (contextId: string) => Promise<void>;
	/** Whether a run is in flight for the context. */
	isRunning: (contextId: string) => boolean;
	/** The live run streaming for the context, if any, for the open transcript. */
	liveRunFor: (contextId: string) => LiveAgentRun | undefined;
};

/**
 * Board-wide wiring for agent runs. Starting and stopping go through the Rust
 * `workspace` use case, which picks the folder, moves the status, captures the
 * conversation id, and persists the transcript. The frontend only asks Rust to
 * run and mirrors the `agent-run` event stream for live display; past runs are
 * read back from Rust per context by the conversation modal.
 */
export function useContextAgentRunSessions(project: ProjectView): ContextAgentRunSessions {
	const { mutateAsync: startAgentRun } = useStartAgentRun(project.tag);
	const { mutateAsync: stopRun } = useStopAgentRun();
	const { liveRunFor, isRunning, notePendingPrompt } = useAgentRunStream();

	const runAgent = useCallback(
		async ({ contextId, prompt }: RunAgentParams) => {
			notePendingPrompt(contextId, prompt);
			await startAgentRun({ contextId, prompt });
		},
		[notePendingPrompt, startAgentRun],
	);

	const stopAgentRun = useCallback(
		async (contextId: string) => {
			await stopRun(contextId);
		},
		[stopRun],
	);

	return { runAgent, stopAgentRun, isRunning, liveRunFor };
}
