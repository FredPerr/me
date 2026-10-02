import { notifications } from "@mantine/notifications";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { ContextStatus } from "@/models/ContextStatus";
import {
	Context,
	ContextBranch,
	Project,
	type PullRequestDraft,
	type Repository,
} from "@/models/Project";
import { ProjectDirectory } from "@/models/ProjectDirectory";

type RepositoryBranchConfig = {
	repositoryId: string;
	createBranch: boolean;
	branchName: string;
	baseBranch: string;
};

type CreateContextParams = {
	name: string;
	repositories: RepositoryBranchConfig[];
	baseContextName?: string;
};

type ExistingBranchConfig = {
	repositoryId: string;
	branch: string | null;
};

type CreateContextFromBranchesParams = {
	name: string;
	repositories: ExistingBranchConfig[];
};

/**
 * One repository's selected pull request within a "create from PRs" operation.
 * `headBranch` is the branch to check out; `baseBranch` is the PR base used as
 * the branch's base. A `null` headBranch links the repository to its current
 * checkout (no PR selected for it).
 */
type PullRequestBranchConfig = {
	repositoryId: string;
	headBranch: string | null;
	baseBranch: string | null;
};

type CreateContextFromPullRequestsParams = {
	name: string;
	repositories: PullRequestBranchConfig[];
};

/**
 * One context to create in a bulk operation. The same branch is used across
 * all effective repositories (created off their default branch), and the
 * optional preprompt is stored on the context for later prefill into Kiro.
 */
type BulkContextSpec = {
	contextName: string;
	branchName: string;
	preprompt?: string;
};

export type {
	BulkContextSpec,
	CreateContextFromBranchesParams,
	CreateContextFromPullRequestsParams,
	CreateContextParams,
	ExistingBranchConfig,
	PullRequestBranchConfig,
	RepositoryBranchConfig,
};

export function useContexts(project: Project) {
	const [defaultContext, setDefaultContext] = useState<Context | null>(null);
	const [persistedContexts, setPersistedContexts] = useState<Context[]>(project.contexts ?? []);

	useEffect(() => {
		setPersistedContexts(project.contexts ?? []);
	}, [project]);

	const hasPersistedDefault = (project.contexts ?? []).some((c) => c.isDefault);

	useEffect(() => {
		if (hasPersistedDefault) {
			setDefaultContext(null);
		} else {
			buildDefaultContext(project).then(setDefaultContext);
		}
	}, [project, hasPersistedDefault]);

	const contexts = defaultContext ? [defaultContext, ...persistedContexts] : persistedContexts;

	const createContext = useCallback(
		async ({ name, repositories: repoConfigs, baseContextName }: CreateContextParams) => {
			const repos = await buildRepoInputs(project, repoConfigs);

			if (repos.length > 0) {
				await invoke("create_context", {
					projectPath: project.path,
					contextName: name,
					repos,
					symlinks: project.symlinks,
					baseContextName: baseContextName ?? null,
				});
			}

			const branches = repoConfigs.map(
				(r) =>
					new ContextBranch(
						r.repositoryId,
						r.createBranch ? r.branchName : r.baseBranch,
						!r.createBranch,
					),
			);

			const newContext = new Context(crypto.randomUUID(), name, branches, false, baseContextName);
			const updatedProject = project.addContext(newContext);

			await ProjectDirectory.saveProject(updatedProject);
			setPersistedContexts((prev) => [...prev, newContext]);
		},
		[project],
	);

	const createContextFromBranches = useCallback(
		async ({ name, repositories: branchConfigs }: CreateContextFromBranchesParams) => {
			const repos = await buildRepoInputsFromExistingBranches(project, branchConfigs);

			if (repos.length > 0) {
				await invoke("create_context", {
					projectPath: project.path,
					contextName: name,
					repos,
					symlinks: project.symlinks,
					baseContextName: null,
				});
			}

			const branches = repos.map(
				(repo) => new ContextBranch(repo.repository_id, repo.branch, repo.linked),
			);

			const newContext = new Context(crypto.randomUUID(), name, branches, false);
			const updatedProject = project.addContext(newContext);

			await ProjectDirectory.saveProject(updatedProject);
			setPersistedContexts((prev) => [...prev, newContext]);
		},
		[project],
	);

	const createContextFromPullRequests = useCallback(
		async ({ name, repositories: prConfigs }: CreateContextFromPullRequestsParams) => {
			// Fetch every selected PR head branch first. Open PRs usually live only
			// on the remote, so this makes them resolvable locally. A fetch failure
			// (branch gone, no access, offline) aborts creation so no partial
			// context is left behind.
			await fetchPullRequestBranches(project, prConfigs);

			const repos = await buildRepoInputsFromPullRequests(project, prConfigs);

			if (repos.length > 0) {
				await invoke("create_context", {
					projectPath: project.path,
					contextName: name,
					repos,
					symlinks: project.symlinks,
					baseContextName: null,
				});
			}

			const branches = repos.map(
				(repo) => new ContextBranch(repo.repository_id, repo.branch, repo.linked),
			);

			// A context created from open PRs exists to be reviewed, so it starts
			// in the review column rather than the default ready state.
			const newContext = new Context(
				crypto.randomUUID(),
				name,
				branches,
				false,
				undefined,
				{},
				"review",
			);
			const updatedProject = project.addContext(newContext);

			await ProjectDirectory.saveProject(updatedProject);
			setPersistedContexts((prev) => [...prev, newContext]);
		},
		[project],
	);

	const createContextsBulk = useCallback(
		async (specs: BulkContextSpec[], baseContext?: Context) => {
			const createdContexts: Context[] = [];
			const baseContextName = baseContext?.name;

			for (const spec of specs) {
				const repos = baseContext
					? await buildRepoInputsFromBaseContext(project, baseContext, spec.branchName)
					: await buildRepoInputsForBranch(project, spec.branchName);
				if (repos.length === 0) continue;

				await invoke("create_context", {
					projectPath: project.path,
					contextName: spec.contextName,
					repos,
					symlinks: project.symlinks,
					baseContextName: baseContextName ?? null,
				});

				const branches = repos.map((repo) => new ContextBranch(repo.repository_id, repo.branch));
				createdContexts.push(
					new Context(
						crypto.randomUUID(),
						spec.contextName,
						branches,
						false,
						baseContextName,
						{},
						undefined,
						spec.preprompt,
					),
				);
			}

			if (createdContexts.length === 0) return;

			let updatedProject = project;
			for (const context of createdContexts) {
				updatedProject = updatedProject.addContext(context);
			}

			await ProjectDirectory.saveProject(updatedProject);
			setPersistedContexts((prev) => [...prev, ...createdContexts]);
		},
		[project],
	);

	const deleteContext = useCallback(
		async (contextId: string) => {
			const context = contexts.find((c) => c.id === contextId);
			if (!context) return;

			const repos = await buildRepoInputsFromContext(project, context);

			try {
				await invoke("delete_context", {
					projectPath: project.path,
					contextName: context.name,
					repos,
					symlinks: project.symlinks,
				});
			} catch (error) {
				notifications.show({
					title: "Failed to delete context",
					message: String(error),
					color: "red",
				});
				return;
			}

			const updatedProject = project.removeContext(contextId);
			await ProjectDirectory.saveProject(updatedProject);
			setPersistedContexts((prev) => prev.filter((c) => c.id !== contextId));
		},
		[project, contexts],
	);

	const savePullRequestDrafts = useCallback(
		async (contextId: string, drafts: Record<string, PullRequestDraft>) => {
			const target = persistedContexts.find((c) => c.id === contextId);
			if (!target) return;

			const updatedContext = target.withPullRequestDrafts(drafts);
			const updatedProject = new Project(
				project.name,
				project.tag,
				project.path,
				project.repositories,
				project.contexts.map((c) => (c.id === contextId ? updatedContext : c)),
				project.icon,
				project.symlinks,
				project.remoteProjectLinks,
				project.branchNaming,
			);

			try {
				await ProjectDirectory.saveProject(updatedProject);
			} catch (error) {
				notifications.show({
					title: "Failed to save pull request drafts",
					message: String(error),
					color: "red",
				});
				return;
			}

			setPersistedContexts((prev) => prev.map((c) => (c.id === contextId ? updatedContext : c)));
		},
		[project, persistedContexts],
	);

	const setContextStatus = useCallback(
		async (contextId: string, status: ContextStatus) => {
			if (defaultContext && contextId === defaultContext.id) {
				setDefaultContext(defaultContext.withStatus(status));
				return;
			}

			const target = persistedContexts.find((c) => c.id === contextId);
			if (!target || target.status === status) return;

			const updatedContext = target.withStatus(status);
			const previousContexts = persistedContexts;
			setPersistedContexts((prev) => prev.map((c) => (c.id === contextId ? updatedContext : c)));

			const updatedProject = new Project(
				project.name,
				project.tag,
				project.path,
				project.repositories,
				project.contexts.map((c) => (c.id === contextId ? updatedContext : c)),
				project.icon,
				project.symlinks,
				project.remoteProjectLinks,
				project.branchNaming,
			);

			try {
				await ProjectDirectory.saveProject(updatedProject);
			} catch (error) {
				setPersistedContexts(previousContexts);
				notifications.show({
					title: "Failed to update context status",
					message: String(error),
					color: "red",
				});
			}
		},
		[project, persistedContexts, defaultContext],
	);

	const setContextExpanded = useCallback(
		async (contextId: string, expanded: boolean) => {
			if (defaultContext && contextId === defaultContext.id) {
				setDefaultContext(defaultContext.withExpanded(expanded));
				return;
			}

			const target = persistedContexts.find((c) => c.id === contextId);
			if (!target || target.expanded === expanded) return;

			const updatedContext = target.withExpanded(expanded);
			const previousContexts = persistedContexts;
			setPersistedContexts((prev) => prev.map((c) => (c.id === contextId ? updatedContext : c)));

			const updatedProject = new Project(
				project.name,
				project.tag,
				project.path,
				project.repositories,
				project.contexts.map((c) => (c.id === contextId ? updatedContext : c)),
				project.icon,
				project.symlinks,
				project.remoteProjectLinks,
				project.branchNaming,
			);

			try {
				await ProjectDirectory.saveProject(updatedProject);
			} catch (error) {
				setPersistedContexts(previousContexts);
				notifications.show({
					title: "Failed to update context",
					message: String(error),
					color: "red",
				});
			}
		},
		[project, persistedContexts, defaultContext],
	);

	return {
		contexts,
		defaultContext,
		createContext,
		createContextFromBranches,
		createContextFromPullRequests,
		createContextsBulk,
		deleteContext,
		savePullRequestDrafts,
		setContextStatus,
		setContextExpanded,
	};
}

/**
 * Builds `create_context` repo inputs that create `branchName` as a new branch
 * off each effective repository's default branch. Used by the bulk creator,
 * where every context shares one branch name across all repositories.
 */
async function buildRepoInputsForBranch(project: Project, branchName: string) {
	const inputs = [];
	for (const repo of project.effectiveRepositories()) {
		const resolvedPath = await repo.resolveAbsolutePath(project.path);
		const baseBranch = await resolveDefaultBranch(project, repo);
		inputs.push({
			repository_id: repo.id,
			rel_path: resolvedPath,
			name: repo.name,
			branch: branchName,
			base_branch: baseBranch,
			linked: false,
			post_checkout_command: repo.postCheckoutCommand ?? null,
		});
	}
	return inputs;
}

/**
 * Builds `create_context` repo inputs that create `branchName` as a new branch
 * off the source context's branch for each effective repository. Used by the
 * bulk creator when a base context is chosen, so every new context branches
 * from that context's work instead of each repository's default branch. Falls
 * back to the repository's default branch when the base context has no branch
 * recorded for a repository.
 */
async function buildRepoInputsFromBaseContext(
	project: Project,
	baseContext: Context,
	branchName: string,
) {
	const inputs = [];
	for (const repo of project.effectiveRepositories()) {
		const resolvedPath = await repo.resolveAbsolutePath(project.path);
		const baseBranch =
			baseContext.getBranchForRepository(repo.id) ?? (await resolveDefaultBranch(project, repo));
		inputs.push({
			repository_id: repo.id,
			rel_path: resolvedPath,
			name: repo.name,
			branch: branchName,
			base_branch: baseBranch,
			linked: false,
			post_checkout_command: repo.postCheckoutCommand ?? null,
		});
	}
	return inputs;
}

async function buildDefaultContext(project: Project): Promise<Context | null> {
	if (project.repositories.length === 0) return null;

	const branches: ContextBranch[] = [];

	for (const repo of project.repositories) {
		try {
			const resolvedPath = await repo.resolveAbsolutePath(project.path);
			const branchList = await invoke<string[]>("list_branches", { path: resolvedPath });
			const defaultBranch =
				branchList.find((b) => b === "main") ??
				branchList.find((b) => b === "master") ??
				branchList[0];
			if (defaultBranch) {
				branches.push(new ContextBranch(repo.id, defaultBranch));
			}
		} catch {
			// repo might not exist yet
		}
	}

	if (branches.length === 0) return null;

	return new Context("default", "default", branches, true);
}

async function buildRepoInputs(project: Project, repoConfigs: RepositoryBranchConfig[]) {
	const inputs = [];
	for (const config of repoConfigs) {
		const repo = project.findEffectiveRepository(config.repositoryId);
		if (!repo) continue;
		const resolvedPath = await repo.resolveAbsolutePath(project.path);
		inputs.push({
			rel_path: resolvedPath,
			name: repo.name,
			branch: config.createBranch ? config.branchName : config.baseBranch,
			base_branch: config.baseBranch,
			linked: !config.createBranch,
			post_checkout_command: repo.postCheckoutCommand ?? null,
		});
	}
	return inputs;
}

async function resolveDefaultBranch(project: Project, repo: Repository): Promise<string> {
	try {
		const resolvedPath = await repo.resolveAbsolutePath(project.path);
		const branchList = await invoke<string[]>("list_branches", { path: resolvedPath });
		return (
			branchList.find((b) => b === "main") ??
			branchList.find((b) => b === "master") ??
			branchList[0] ??
			""
		);
	} catch {
		return "";
	}
}

async function buildRepoInputsFromExistingBranches(
	project: Project,
	branchConfigs: ExistingBranchConfig[],
) {
	const inputs = [];
	for (const config of branchConfigs) {
		const repo = project.findRepository(config.repositoryId);
		if (!repo) continue;

		const resolvedPath = await repo.resolveAbsolutePath(project.path);
		const isLinked = !config.branch;
		const branch = config.branch ?? (await resolveDefaultBranch(project, repo));

		inputs.push({
			repository_id: repo.id,
			rel_path: resolvedPath,
			name: repo.name,
			branch,
			base_branch: branch,
			linked: isLinked,
			post_checkout_command: repo.postCheckoutCommand ?? null,
		});
	}
	return inputs;
}

/**
 * Fetches each selected PR head branch into its repository so a worktree can be
 * checked out to it. Repositories with no selected PR are skipped. Rejects on
 * the first fetch failure so the caller can surface the error and abort before
 * any context is created.
 */
async function fetchPullRequestBranches(
	project: Project,
	prConfigs: PullRequestBranchConfig[],
): Promise<void> {
	for (const config of prConfigs) {
		if (!config.headBranch) continue;

		const repo = project.findEffectiveRepository(config.repositoryId);
		if (!repo) continue;

		const resolvedPath = await repo.resolveAbsolutePath(project.path);
		await invoke("fetch_pr_branch", { path: resolvedPath, branch: config.headBranch });
	}
}

/**
 * Builds `create_context` repo inputs from selected pull requests. A repository
 * with a selected PR checks out the PR head branch (based on the PR base
 * branch); a repository without one is linked to its current checkout, matching
 * the "create from branches" behavior.
 */
async function buildRepoInputsFromPullRequests(
	project: Project,
	prConfigs: PullRequestBranchConfig[],
) {
	const inputs = [];
	for (const config of prConfigs) {
		const repo = project.findEffectiveRepository(config.repositoryId);
		if (!repo) continue;

		const resolvedPath = await repo.resolveAbsolutePath(project.path);
		const isLinked = !config.headBranch;
		const branch = config.headBranch ?? (await resolveDefaultBranch(project, repo));
		const baseBranch = config.baseBranch ?? branch;

		inputs.push({
			repository_id: repo.id,
			rel_path: resolvedPath,
			name: repo.name,
			branch,
			base_branch: baseBranch,
			linked: isLinked,
			post_checkout_command: repo.postCheckoutCommand ?? null,
		});
	}
	return inputs;
}

async function buildRepoInputsFromContext(project: Project, context: Context) {
	const inputs = [];
	for (const contextBranch of context.branches) {
		const repo = project.findEffectiveRepository(contextBranch.repositoryId);
		if (!repo) continue;
		const resolvedPath = await repo.resolveAbsolutePath(project.path);
		inputs.push({
			rel_path: resolvedPath,
			name: repo.name,
			branch: contextBranch.branch,
			base_branch: "",
			linked: contextBranch.linked,
		});
	}
	return inputs;
}
