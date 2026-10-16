import { useCallback, useEffect, useState } from "react";
import { commands, type ProjectView, type RepositoryView } from "@/bindings";
import type { ProviderPullRequest } from "@/models/git-provider/GitProvider";
import {
	listGitProviders,
	resolveProviderForRemote,
} from "@/models/git-provider/GitProviderRegistry";

/**
 * Looks up the pull request matching a repository branch within a project.
 * Returns null when no provider is connected or no pull request matches.
 */
export type PullRequestLookup = (
	repositoryId: string,
	branch: string,
) => ProviderPullRequest | null;

type UseContextPullRequestStatusResult = {
	connected: boolean;
	lookup: PullRequestLookup;
};

/** Pull requests for one repository, keyed by head branch. */
type RepositoryPullRequestIndex = Map<string, ProviderPullRequest>;

async function resolveRemoteUrl(repository: RepositoryView): Promise<string | null> {
	try {
		return await commands.getGitRemoteUrl(repository.path);
	} catch {
		return null;
	}
}

/**
 * Index pull requests by head branch, keeping the most recently updated one
 * when several share a branch (reopened or recreated pull requests).
 */
function indexByHeadBranch(pullRequests: ProviderPullRequest[]): RepositoryPullRequestIndex {
	const index: RepositoryPullRequestIndex = new Map();
	for (const pullRequest of pullRequests) {
		const existing = index.get(pullRequest.headBranch);
		if (!existing || pullRequest.updatedAt > existing.updatedAt) {
			index.set(pullRequest.headBranch, pullRequest);
		}
	}
	return index;
}

async function loadRepositoryIndex(
	repository: RepositoryView,
): Promise<[string, RepositoryPullRequestIndex] | null> {
	const remoteUrl = await resolveRemoteUrl(repository);
	if (!remoteUrl) return null;

	const resolved = resolveProviderForRemote(remoteUrl);
	if (!resolved) return null;

	const { provider, ref } = resolved;
	if (!(await provider.isConnected())) return null;

	const pullRequests = await provider.listPullRequests(ref);
	return [repository.id, indexByHeadBranch(pullRequests)];
}

/**
 * Resolves the pull request state for every repository in a project so context
 * cards can show a provider status icon per sub-repo branch. Fetches once per
 * repository and only when the owning provider is connected.
 */
export function useContextPullRequestStatus(
	project: ProjectView,
): UseContextPullRequestStatusResult {
	const [connected, setConnected] = useState(false);
	const [indexes, setIndexes] = useState<Map<string, RepositoryPullRequestIndex>>(new Map());

	const reload = useCallback(async () => {
		const connectedFlags = await Promise.all(
			listGitProviders().map((provider) => provider.isConnected()),
		);
		const anyConnected = connectedFlags.some(Boolean);
		setConnected(anyConnected);
		if (!anyConnected) {
			setIndexes(new Map());
			return;
		}

		const results = await Promise.all(
			project.effectiveRepositories.map((repository) =>
				loadRepositoryIndex(repository).catch(() => null),
			),
		);
		const next = new Map<string, RepositoryPullRequestIndex>();
		for (const result of results) {
			if (result) next.set(result[0], result[1]);
		}
		setIndexes(next);
	}, [project.effectiveRepositories]);

	useEffect(() => {
		reload().catch(() => {
			setConnected(false);
			setIndexes(new Map());
		});
	}, [reload]);

	const lookup = useCallback<PullRequestLookup>(
		(repositoryId, branch) => indexes.get(repositoryId)?.get(branch) ?? null,
		[indexes],
	);

	return { connected, lookup };
}
