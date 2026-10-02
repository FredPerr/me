import { BitbucketProvider } from "./BitbucketProvider";
import { GitHubProvider } from "./GitHubProvider";
import type { GitProvider, GitProviderId, RepositoryRef } from "./GitProvider";

/**
 * Resolves a {@link GitProvider} implementation by id. Central place to
 * register new providers (GitLab, …) as they are added.
 */
const PROVIDERS: Record<GitProviderId, GitProvider> = {
	github: new GitHubProvider(),
	bitbucket: new BitbucketProvider(),
};

export function getGitProvider(id: GitProviderId): GitProvider {
	return PROVIDERS[id];
}

export function listGitProviders(): GitProvider[] {
	return Object.values(PROVIDERS);
}

/**
 * Find the provider that owns the given remote URL, along with the parsed
 * repository ref, or null when no registered provider recognizes the host.
 * Lets callers route an operation to the right provider based on the remote.
 */
export function resolveProviderForRemote(
	remoteUrl: string,
): { provider: GitProvider; ref: RepositoryRef } | null {
	for (const provider of listGitProviders()) {
		const ref = provider.parseRepositoryRef(remoteUrl);
		if (ref) {
			return { provider, ref };
		}
	}
	return null;
}
