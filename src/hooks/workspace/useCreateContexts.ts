import { useCallback } from "react";
import type { ContextView, ProjectView } from "@/bindings";
import type { BulkContextSpec } from "@/models/bulk/BulkContextSpec";
import { newBranchesFrom } from "@/models/projectView";
import { useSuggestBranchName } from "./useBranchNameSuggestion";
import { useCreateContext } from "./useContextMutations";

/**
 * Creates the contexts one by one, each from `baseContext` (or the
 * repositories' default branches), and stops at the first failure.
 * Resolves to the number of contexts created.
 */
export function useCreateContexts(project: ProjectView) {
	const { mutateAsync: createContext, isPending: isCreating } = useCreateContext();
	const { mutateAsync: suggestBranchName, isPending: isSuggesting } = useSuggestBranchName(project);

	const createContexts = useCallback(
		async (specs: readonly BulkContextSpec[], baseContext?: ContextView): Promise<number> => {
			let createdCount = 0;
			for (const spec of specs) {
				const branchName =
					spec.branchName?.trim() || (await suggestBranchName({ title: spec.contextName }));
				await createContext({
					projectTag: project.tag,
					name: spec.contextName,
					baseContextId: baseContext?.id ?? null,
					origin: spec.task ? { kind: "fromTask", ...spec.task } : { kind: "manual" },
					status: null,
					agentPrompt: spec.agentPrompt ?? null,
					branches: newBranchesFrom(project, baseContext, branchName),
				});
				createdCount += 1;
			}
			return createdCount;
		},
		[project, createContext, suggestBranchName],
	);

	return { createContexts, creating: isCreating || isSuggesting };
}
