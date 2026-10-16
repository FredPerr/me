import { useCallback, useState } from "react";
import type { ContextView } from "@/bindings";
import { useSetAgentPrompt } from "@/hooks/workspace/useContextMutations";

/**
 * The prompt typed in each context's Kiro conversation, starting from the
 * context's saved agent prompt. `keepPrompt` saves an edited prompt on the
 * context so it is still there next time.
 */
export function useKiroPromptDrafts(projectTag: string) {
	const [promptDrafts, setPromptDrafts] = useState<Record<string, string>>({});
	const { mutate: setAgentPrompt } = useSetAgentPrompt(projectTag);

	const promptFor = useCallback(
		(context: ContextView) => promptDrafts[context.id] ?? context.agentWork?.prompt ?? "",
		[promptDrafts],
	);

	const changePrompt = useCallback((contextId: string, prompt: string) => {
		setPromptDrafts((current) => ({ ...current, [contextId]: prompt }));
	}, []);

	const keepPrompt = useCallback(
		(context: ContextView) => {
			const draft = promptDrafts[context.id];
			if (draft === undefined || draft === (context.agentWork?.prompt ?? "")) return;
			setAgentPrompt({ contextId: context.id, prompt: draft.trim() === "" ? null : draft });
		},
		[promptDrafts, setAgentPrompt],
	);

	return { promptFor, changePrompt, keepPrompt };
}
