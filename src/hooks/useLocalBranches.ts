import { useCallback, useEffect, useState } from "react";
import { commands, type LocalBranchView, type ProjectView, type RepositoryView } from "@/bindings";
import { describeError } from "@/infra/commandError";

export type LocalBranch = LocalBranchView;

export type RepositoryBranches = {
	repository: RepositoryView;
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

export function useLocalBranches(project: ProjectView, enabled: boolean) {
	const [repositoryBranches, setRepositoryBranches] = useState<RepositoryBranches[]>([]);
	const [loading, setLoading] = useState(false);

	const load = useCallback(async () => {
		setLoading(true);
		try {
			const entries = await Promise.all(
				project.effectiveRepositories.map(async (repository) => {
					try {
						const branches = await commands.listLocalBranches(repository.path);
						return { repository, resolvedPath: repository.path, branches };
					} catch {
						return { repository, resolvedPath: "", branches: [] };
					}
				}),
			);
			setRepositoryBranches(entries);
		} finally {
			setLoading(false);
		}
	}, [project.effectiveRepositories]);

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
						await commands.deleteLocalBranch(selection.resolvedPath, selection.branchName, true);
						return null;
					} catch (error) {
						return {
							selection,
							message: describeError(error),
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

		await Promise.all(paths.map((path) => commands.pruneWorktrees(path).catch(() => undefined)));
		await load();
	}, [repositoryBranches, load]);

	return { repositoryBranches, loading, reload: load, deleteBranches, pruneWorktrees };
}
