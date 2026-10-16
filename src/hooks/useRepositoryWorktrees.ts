import { useCallback, useEffect, useState } from "react";
import { commands, type WorktreeView } from "@/bindings";
import { describeError } from "@/infra/commandError";
import type { Project, Repository } from "@/models/Project";

export type Worktree = WorktreeView;

export type RepositoryWorktrees = {
	repository: Repository;
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

export function useRepositoryWorktrees(project: Project, enabled: boolean) {
	const [repositoryWorktrees, setRepositoryWorktrees] = useState<RepositoryWorktrees[]>([]);
	const [loading, setLoading] = useState(false);

	const load = useCallback(async () => {
		setLoading(true);
		try {
			const entries = await Promise.all(
				project.repositories.map(async (repository) => {
					try {
						const resolvedPath = await repository.resolveAbsolutePath(project.path);
						const worktrees = await commands.listWorktrees(resolvedPath);
						return { repository, resolvedPath, worktrees };
					} catch {
						return { repository, resolvedPath: "", worktrees: [] };
					}
				}),
			);
			setRepositoryWorktrees(entries);
		} finally {
			setLoading(false);
		}
	}, [project]);

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
