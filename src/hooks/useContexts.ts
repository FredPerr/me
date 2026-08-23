import { notifications } from "@mantine/notifications";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { Context, ContextBranch, type Project } from "@/models/Project";
import { ProjectDirectory } from "@/models/ProjectDirectory";

type CreateContextParams = {
	name: string;
	branchName: string;
	baseBranches: Record<string, string>;
};

export function useContexts(project: Project) {
	const [defaultContext, setDefaultContext] = useState<Context | null>(null);
	const [persistedContexts, setPersistedContexts] = useState<Context[]>(project.contexts ?? []);

	useEffect(() => {
		setPersistedContexts(project.contexts ?? []);
	}, [project]);

	useEffect(() => {
		buildDefaultContext(project).then(setDefaultContext);
	}, [project]);

	const contexts = defaultContext
		? [defaultContext, ...persistedContexts]
		: persistedContexts;

	const createContext = useCallback(
		async ({ name, branchName, baseBranches }: CreateContextParams) => {
			const repos = await buildRepoInputs(project, branchName, baseBranches);

			await invoke("create_context", {
				projectPath: project.path,
				contextName: name,
				repos,
			});

			const newContext = Context.create(name, project.repositories, branchName);
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

	return { contexts, defaultContext, createContext, deleteContext };
}

async function buildDefaultContext(project: Project): Promise<Context | null> {
	if (project.repositories.length === 0) return null;

	const branches: ContextBranch[] = [];

	for (const repo of project.repositories) {
		try {
			const resolvedPath = await repo.resolveAbsolutePath(project.path);
			const branchList = await invoke<string[]>("list_branches", { path: resolvedPath });
			const defaultBranch = branchList.find((b) => b === "main") ?? branchList.find((b) => b === "master") ?? branchList[0];
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

async function buildRepoInputs(
	project: Project,
	branchName: string,
	baseBranches: Record<string, string>,
) {
	const inputs = [];
	for (const repo of project.repositories) {
		const resolvedPath = await repo.resolveAbsolutePath(project.path);
		inputs.push({
			rel_path: resolvedPath,
			name: repo.name,
			branch: branchName,
			base_branch: baseBranches[repo.id] ?? "main",
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
		});
	}
	return inputs;
}
