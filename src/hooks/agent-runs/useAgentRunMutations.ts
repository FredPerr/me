import { useMutation } from "@tanstack/react-query";
import { commands } from "@/bindings";

type StartAgentRunInput = {
	contextId: string;
	prompt: string;
};

/**
 * Starts an agent run on a context through the Rust use case. Rust picks the
 * folder, moves the status, and persists the transcript; the frontend only asks
 * it to run. Status changes arrive back through the `project-changed` event.
 */
export function useStartAgentRun(projectTag: string) {
	return useMutation({
		mutationFn: ({ contextId, prompt }: StartAgentRunInput) =>
			commands.workspaceRunAgent(projectTag, contextId, prompt),
	});
}

/** Stops the running agent for a context. Rust records the stop and moves the status. */
export function useStopAgentRun() {
	return useMutation({
		mutationFn: (contextId: string) => commands.workspaceStopAgentRun(contextId),
	});
}
