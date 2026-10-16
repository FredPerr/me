import type { BranchPullRequestView, HostedPullRequestView } from "@/bindings";

/** The pull request of a context branch in one repository, or null when it has none or it is unknown. */
export type PullRequestLookup = (
	repositoryId: string,
	branch: string,
) => HostedPullRequestView | null;

function lookupKey(repositoryId: string, branch: string): string {
	return JSON.stringify([repositoryId, branch]);
}

export function branchPullRequestLookup(found: BranchPullRequestView[]): PullRequestLookup {
	const pullRequests = new Map<string, HostedPullRequestView>();
	for (const { repositoryId, branch, status } of found) {
		if (status.kind === "ok" && status.pullRequest) {
			pullRequests.set(lookupKey(repositoryId, branch), status.pullRequest);
		}
	}
	return (repositoryId, branch) => pullRequests.get(lookupKey(repositoryId, branch)) ?? null;
}

/** Whether the user is signed in to the Git host of at least one context branch. */
export function isAnyBranchHostConnected(found: BranchPullRequestView[]): boolean {
	return found.some(({ status }) => status.kind === "ok" || status.kind === "failed");
}
