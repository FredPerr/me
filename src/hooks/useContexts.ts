import { notifications } from "@mantine/notifications";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { Context, ContextBranch, Project, type PullRequestDraft } from "@/models/Project";
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

export type { CreateContextParams, RepositoryBranchConfig };

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

	return { contexts, defaultContext, createContext, deleteContext, savePullRequestDrafts };
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
		const repo = project.findRepository(config.repositoryId);
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

async function buildRepoInputsFromContext(project: Project, context: Context) {
	const inputs = [];
	for (const contextBranch of context.branches) {
		const repo = project.findRepository(contextBranch.repositoryId);
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
