import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { Project, Repository } from "@/models/Project";

export type Worktree = {
	name: string;
	path: string;
	branch: string | null;
	isDefault: boolean;
};

type WorktreeDto = {
	name: string;
	path: string;
	branch: string | null;
	is_default: boolean;
};

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

function toWorktree(dto: WorktreeDto): Worktree {
	return {
		name: dto.name,
		path: dto.path,
		branch: dto.branch,
		isDefault: dto.is_default,
	};
}

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
						const dtos = await invoke<WorktreeDto[]>("list_worktrees", {
							path: resolvedPath,
						});
						return { repository, resolvedPath, worktrees: dtos.map(toWorktree) };
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
						await invoke("delete_worktree", {
							repoPath: selection.repositoryPath,
							worktreePath: selection.worktreePath,
						});
						return null;
					} catch (error) {
						return {
							selection,
							message: error instanceof Error ? error.message : String(error),
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
