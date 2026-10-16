import { useEffect, useState } from "react";
import { commands, type WorktreeStatusView } from "@/bindings";
import type { Context, Project } from "@/models/Project";

export type RepositoryDiffStats = Omit<WorktreeStatusView, "prUrl">;

export function useContextDiffStats(context: Context, project: Project, allContexts: Context[]) {
	const [statsByRepository, setStatsByRepository] = useState<
		Record<string, RepositoryDiffStats | null>
	>({});

	const hasBaseContext = !!context.baseContextName;

	useEffect(() => {
		if (context.isDefault || !hasBaseContext) return;

		let cancelled = false;

		async function fetchStats() {
			const results: Record<string, RepositoryDiffStats | null> = {};

			const baseContext = allContexts.find((c) => c.name === context.baseContextName);

			for (const branch of context.branches) {
				const repository = project.findRepository(branch.repositoryId);
				if (!repository) continue;

				const path = context.getWorktreePath(project.path, repository);
				const baseBranch = baseContext?.getBranchForRepository(branch.repositoryId) ?? null;

				if (!baseBranch) {
					results[branch.repositoryId] = null;
					continue;
				}

				try {
					const status = await commands.getWorktreeStatus(path, baseBranch);

					const { prUrl: _prUrl, ...stats } = status;
					results[branch.repositoryId] = stats;
				} catch {
					results[branch.repositoryId] = null;
				}
			}

			if (!cancelled) {
				setStatsByRepository(results);
			}
		}

		fetchStats();

		return () => {
			cancelled = true;
		};
	}, [context, project, allContexts, hasBaseContext]);

	return { statsByRepository, hasBaseContext };
}
