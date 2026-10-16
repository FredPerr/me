import { useEffect, useState } from "react";
import { type ContextView, commands, type ProjectView, type WorktreeStatusView } from "@/bindings";
import { findBaseContext, findContextBranch } from "@/models/projectView";

export function useContextDiffStats(context: ContextView, project: ProjectView) {
	const [statsByRepository, setStatsByRepository] = useState<
		Record<string, WorktreeStatusView | null>
	>({});
	const baseContext = findBaseContext(project, context);
	const hasBaseContext = baseContext !== undefined;

	useEffect(() => {
		if (context.isDefault || !baseContext) return;

		let cancelled = false;

		async function fetchStats(base: ContextView) {
			const results: Record<string, WorktreeStatusView | null> = {};

			for (const branch of context.branches) {
				const baseBranch = findContextBranch(base, branch.repositoryId)?.branch;
				if (!branch.worktreePath || !baseBranch) {
					results[branch.repositoryId] = null;
					continue;
				}
				try {
					results[branch.repositoryId] = await commands.getWorktreeStatus(
						branch.worktreePath,
						baseBranch,
					);
				} catch {
					results[branch.repositoryId] = null;
				}
			}

			if (!cancelled) {
				setStatsByRepository(results);
			}
		}

		fetchStats(baseContext);
		return () => {
			cancelled = true;
		};
	}, [context, baseContext]);

	return { statsByRepository, hasBaseContext };
}
