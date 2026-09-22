import { GitHubProvider } from "./GitHubProvider";
import type { GitProvider, GitProviderId } from "./GitProvider";

/**
 * Resolves a {@link GitProvider} implementation by id. Central place to
 * register new providers (GitLab, Bitbucket) as they are added.
 */
const PROVIDERS: Record<GitProviderId, GitProvider> = {
	github: new GitHubProvider(),
};

export function getGitProvider(id: GitProviderId): GitProvider {
	return PROVIDERS[id];
}

export function listGitProviders(): GitProvider[] {
	return Object.values(PROVIDERS);
}
