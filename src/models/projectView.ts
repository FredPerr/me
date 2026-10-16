import type {
	ContextBranchView,
	ContextView,
	ProjectView,
	RepositoryBranchInput,
	RepositoryView,
} from "@/bindings";

/** Identifies the remote task a context was created from. */
export type TaskRef = {
	connectionId: string;
	taskId: string;
};

export function findContext(project: ProjectView, contextId: string): ContextView | undefined {
	return project.contexts.find((context) => context.id === contextId);
}

export function findDefaultContext(project: ProjectView): ContextView | undefined {
	return project.contexts.find((context) => context.isDefault);
}

export function findBaseContext(
	project: ProjectView,
	context: ContextView,
): ContextView | undefined {
	return context.baseContextId === null ? undefined : findContext(project, context.baseContextId);
}

export function findEffectiveRepository(
	project: ProjectView,
	repositoryId: string,
): RepositoryView | undefined {
	return project.effectiveRepositories.find((repository) => repository.id === repositoryId);
}

export function findContextBranch(
	context: ContextView,
	repositoryId: string,
): ContextBranchView | undefined {
	return context.branches.find((branch) => branch.repositoryId === repositoryId);
}

/** A repository's default branch: the default context's branch in it. */
export function findDefaultBranch(project: ProjectView, repositoryId: string): string | undefined {
	const defaultContext = findDefaultContext(project);
	return defaultContext ? findContextBranch(defaultContext, repositoryId)?.branch : undefined;
}

export function isCreatedFromTask(context: ContextView, task: TaskRef): boolean {
	return (
		context.origin.kind === "fromTask" &&
		context.origin.connectionId === task.connectionId &&
		context.origin.taskId === task.taskId
	);
}

/** Every worktree folder of the project's contexts, each once. */
export function collectWorktreePaths(project: ProjectView): string[] {
	const paths = project.contexts.flatMap((context) =>
		context.branches.flatMap((branch) => (branch.worktreePath ? [branch.worktreePath] : [])),
	);
	return [...new Set(paths)];
}

/**
 * The same new branch in every repository, started from the base context's
 * branch there, or from the repository's default branch.
 */
export function newBranchesFrom(
	project: ProjectView,
	baseContext: ContextView | undefined,
	branchName: string,
): RepositoryBranchInput[] {
	return project.effectiveRepositories.map((repository) => {
		const baseBranch =
			(baseContext && findContextBranch(baseContext, repository.id)?.branch) ??
			findDefaultBranch(project, repository.id) ??
			"";
		return {
			repositoryId: repository.id,
			branch: { kind: "newBranch", name: branchName, baseBranch },
		};
	});
}
