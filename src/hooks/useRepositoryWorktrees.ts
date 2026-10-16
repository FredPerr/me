import { useCallback, useEffect, useState } from "react";
import { commands, type ProjectView, type RepositoryView, type WorktreeView } from "@/bindings";
import { describeError } from "@/infra/commandError";

export type Worktree = WorktreeView;

export type RepositoryWorktrees = {
	repository: RepositoryView;
	resolvedPath: string;
	worktrees: Worktree[];
};

export type WorktreeSelection = {
	repositoryPath: string;
	worktreePath: string;
};

export type WorktreeDeletionFailure = {
	selection: WorktreeSelection;
	message: string;
};

export function useRepositoryWorktrees(project: ProjectView, enabled: boolean) {
	const [repositoryWorktrees, setRepositoryWorktrees] = useState<RepositoryWorktrees[]>([]);
	const [loading, setLoading] = useState(false);

	const load = useCallback(async () => {
		setLoading(true);
		try {
			const entries = await Promise.all(
				project.effectiveRepositories.map(async (repository) => {
					try {
						const worktrees = await commands.listWorktrees(repository.path);
						return { repository, resolvedPath: repository.path, worktrees };
					} catch {
						return { repository, resolvedPath: "", worktrees: [] };
					}
				}),
			);
			setRepositoryWorktrees(entries);
		} finally {
			setLoading(false);
		}
	}, [project.effectiveRepositories]);

	useEffect(() => {
		if (enabled) {
			load();
		}
	}, [enabled, load]);

	const deleteWorktrees = useCallback(
		async (selections: WorktreeSelection[]): Promise<WorktreeDeletionFailure[]> => {
			if (selections.length === 0) return [];

			const outcomes = await Promise.all(
				selections.map(async (selection) => {
					try {
						await commands.deleteWorktree(selection.repositoryPath, selection.worktreePath);
						return null;
					} catch (error) {
						return {
							selection,
							message: describeError(error),
						} satisfies WorktreeDeletionFailure;
					}
				}),
			);

			await load();

			return outcomes.filter((outcome): outcome is WorktreeDeletionFailure => !!outcome);
		},
		[load],
	);

	return { repositoryWorktrees, loading, reload: load, deleteWorktrees };
}
