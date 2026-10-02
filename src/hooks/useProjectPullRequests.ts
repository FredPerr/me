import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { GitProviderId, ProviderPullRequest } from "@/models/git-provider/GitProvider";
import {
	listGitProviders,
	resolveProviderForRemote,
} from "@/models/git-provider/GitProviderRegistry";
import { type Project, Repository } from "@/models/Project";

/** Open pull requests for one repository within a project. */
export type RepositoryPullRequests = {
	project: Project;
	repository: Repository;
	providerId: GitProviderId;
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
		const remoteUrl = await invoke<string | null>("get_git_remote_url", { path: resolvedPath });
		if (!remoteUrl) {
			console.warn(
				`[pull-requests] no git remote URL for ${project.name}/${repository.name} at ${resolvedPath}`,
			);
		}
		return remoteUrl;
	} catch (error) {
		console.error(
			`[pull-requests] failed to resolve remote URL for ${project.name}/${repository.name}:`,
			error,
		);
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
	console.debug(
		`[pull-requests] considering ${project.name}/${repository.name} (relPath "${repository.relPath}") -> remote ${remoteUrl ?? "<none>"}`,
	);
	if (!remoteUrl) return null;

	const resolved = resolveProviderForRemote(remoteUrl);
	if (!resolved) {
		console.warn(
			`[pull-requests] remote URL "${remoteUrl}" for ${project.name}/${repository.name} is not a recognized provider URL; skipping`,
		);
		return null;
	}

	const { provider, ref } = resolved;

	// Only query providers the user has actually connected.
	if (!(await provider.isConnected())) {
		console.debug(
			`[pull-requests] ${provider.id} not connected; skipping ${project.name}/${repository.name}`,
		);
		return null;
	}

	const pullRequests = await provider.listPullRequests(ref);
	console.debug(
		`[pull-requests] ${provider.id} ${ref.owner}/${ref.repo} returned ${pullRequests.length} open PR(s)`,
	);
	return { project, repository, providerId: provider.id, pullRequests };
}

/** The result of attempting to load one repository's pull requests. */
type RepositoryLoadResult =
	| { ok: true; group: RepositoryPullRequests | null }
	| { ok: false; error: string };

/**
 * Load one repository's pull requests, capturing any failure as a value so a
 * single failing repository never discards the results of the others.
 */
async function loadRepositorySafely(
	project: Project,
	repository: Repository,
): Promise<RepositoryLoadResult> {
	try {
		const group = await loadRepositoryPullRequests(project, repository);
		return { ok: true, group };
	} catch (error) {
		console.error(
			`[pull-requests] listPullRequests failed for ${project.name}/${repository.name}:`,
			error,
		);
		return { ok: false, error: String(error) };
	}
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
			const connectedFlags = await Promise.all(
				listGitProviders().map((provider) => provider.isConnected()),
			);
			const anyConnected = connectedFlags.some(Boolean);
			console.debug(`[pull-requests] any provider connected -> ${anyConnected}`);
			setConnected(anyConnected);
			if (!anyConnected) {
				setGroups([]);
				return;
			}

			const tasks = projects.flatMap((project) =>
				projectRepositories(project).map((repository) => loadRepositorySafely(project, repository)),
			);
			const results = await Promise.all(tasks);

			const loadedGroups = results.flatMap((result) =>
				result.ok && result.group ? [result.group] : [],
			);
			setGroups(loadedGroups);

			// Surface the first failure, if any, without hiding the repositories
			// that did load successfully.
			const firstFailure = results.find((result) => !result.ok);
			setError(firstFailure && !firstFailure.ok ? firstFailure.error : null);
		} catch (loadError) {
			console.error("[pull-requests] failed to load pull requests:", loadError);
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
