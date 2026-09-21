import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { Project, Repository } from "@/models/Project";

export type LocalBranch = {
	name: string;
	isCurrent: boolean;
	isCheckedOut: boolean;
	hasUpstream: boolean;
	upstreamGone: boolean;
	isMerged: boolean;
};

type LocalBranchDto = {
	name: string;
	is_current: boolean;
	is_checked_out: boolean;
	has_upstream: boolean;
	upstream_gone: boolean;
	is_merged: boolean;
};

export type RepositoryBranches = {
	repository: Repository;
	resolvedPath: string;
	branches: LocalBranch[];
};

export type BranchSelection = {
	resolvedPath: string;
	branchName: string;
};

export type BranchDeletionFailure = {
	selection: BranchSelection;
	message: string;
};

function toLocalBranch(dto: LocalBranchDto): LocalBranch {
	return {
		name: dto.name,
		isCurrent: dto.is_current,
		isCheckedOut: dto.is_checked_out,
		hasUpstream: dto.has_upstream,
		upstreamGone: dto.upstream_gone,
		isMerged: dto.is_merged,
	};
}

export function useLocalBranches(project: Project, enabled: boolean) {
	const [repositoryBranches, setRepositoryBranches] = useState<RepositoryBranches[]>([]);
	const [loading, setLoading] = useState(false);

	const load = useCallback(async () => {
		setLoading(true);
		try {
			const entries = await Promise.all(
				project.repositories.map(async (repository) => {
					try {
						const resolvedPath = await repository.resolveAbsolutePath(project.path);
						const dtos = await invoke<LocalBranchDto[]>("list_local_branches", {
							path: resolvedPath,
						});
						return { repository, resolvedPath, branches: dtos.map(toLocalBranch) };
					} catch {
						return { repository, resolvedPath: "", branches: [] };
					}
				}),
			);
			setRepositoryBranches(entries);
		} finally {
			setLoading(false);
		}
	}, [project]);

	useEffect(() => {
		if (enabled) {
			load();
		}
	}, [enabled, load]);

	const deleteBranches = useCallback(
		async (selections: BranchSelection[]): Promise<BranchDeletionFailure[]> => {
			if (selections.length === 0) return [];

			const selectedKeys = new Set(selections.map((s) => `${s.resolvedPath}:${s.branchName}`));

			setRepositoryBranches((current) =>
				current.map((entry) => ({
					...entry,
					branches: entry.branches.filter(
						(branch) => !selectedKeys.has(`${entry.resolvedPath}:${branch.name}`),
					),
				})),
			);

			const outcomes = await Promise.all(
				selections.map(async (selection) => {
					try {
						await invoke("delete_local_branch", {
							path: selection.resolvedPath,
							branch: selection.branchName,
							force: true,
						});
						return null;
					} catch (error) {
						return {
							selection,
							message: error instanceof Error ? error.message : String(error),
						} satisfies BranchDeletionFailure;
					}
				}),
			);

			const failures = outcomes.filter((outcome): outcome is BranchDeletionFailure => !!outcome);

			if (failures.length > 0) {
				// Restore only the branches that failed to delete.
				await load();
			}

			return failures;
		},
		[load],
	);

	const pruneWorktrees = useCallback(async () => {
		const paths = repositoryBranches
			.map((entry) => entry.resolvedPath)
			.filter((resolvedPath) => resolvedPath.length > 0);

		await Promise.all(
			paths.map((path) => invoke("prune_worktrees", { path }).catch(() => undefined)),
		);
		await load();
	}, [repositoryBranches, load]);

	return { repositoryBranches, loading, reload: load, deleteBranches, pruneWorktrees };
}
