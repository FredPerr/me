import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { ProviderPullRequest } from "@/models/git-provider/GitProvider";
import { getGitProvider } from "@/models/git-provider/GitProviderRegistry";
import { type Project, Repository } from "@/models/Project";

const github = getGitProvider("github");

/** Open pull requests for one repository within a project. */
export type RepositoryPullRequests = {
	project: Project;
	repository: Repository;
	pullRequests: ProviderPullRequest[];
};

type UseProjectPullRequestsResult = {
	connected: boolean;
	loading: boolean;
	groups: RepositoryPullRequests[];
	error: string | null;
	reload: () => Promise<void>;
};

async function resolveRemoteUrl(project: Project, repository: Repository): Promise<string | null> {
	try {
		const resolvedPath = await repository.resolveAbsolutePath(project.path);
		return await invoke<string | null>("get_git_remote_url", { path: resolvedPath });
	} catch {
		return null;
	}
}

function projectRepositories(project: Project): Repository[] {
	// Fall back to the project root as a single repository when no explicit
	// repositories are configured (mono-repo projects).
	return project.repositories.length > 0
		? project.repositories
		: [new Repository(project.tag, project.name, ".")];
}

async function loadRepositoryPullRequests(
	project: Project,
	repository: Repository,
): Promise<RepositoryPullRequests | null> {
	const remoteUrl = await resolveRemoteUrl(project, repository);
	if (!remoteUrl) return null;

	const ref = github.parseRepositoryRef(remoteUrl);
	if (!ref) return null;

	const pullRequests = await github.listPullRequests(ref);
	return { project, repository, pullRequests };
}

/**
 * Resolves every repository in the given projects to its GitHub remote and
 * loads the open pull requests, grouped by repository. Only runs when a GitHub
 * account is connected.
 */
export function useProjectPullRequests(projects: Project[]): UseProjectPullRequestsResult {
	const [connected, setConnected] = useState(false);
	const [loading, setLoading] = useState(true);
	const [groups, setGroups] = useState<RepositoryPullRequests[]>([]);
	const [error, setError] = useState<string | null>(null);

	const reload = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			const isConnected = await github.isConnected();
			setConnected(isConnected);
			if (!isConnected) {
				setGroups([]);
				return;
			}

			const tasks = projects.flatMap((project) =>
				projectRepositories(project).map((repository) =>
					loadRepositoryPullRequests(project, repository),
				),
			);
			const results = await Promise.all(tasks);
			setGroups(results.filter((group): group is RepositoryPullRequests => group !== null));
		} catch (loadError) {
			setError(String(loadError));
			setGroups([]);
		} finally {
			setLoading(false);
		}
	}, [projects]);

	useEffect(() => {
		reload();
	}, [reload]);

	return { connected, loading, groups, error, reload };
}
