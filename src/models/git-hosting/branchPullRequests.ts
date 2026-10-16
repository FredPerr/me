import type { BranchPullRequestView, HostedPullRequestView, HostKindView } from "@/bindings";

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

/** Why no pull request can be created from a context branch, as far as the last lookup knows. */
export type BranchHostProblem =
	| { kind: "noRemote" }
	| { kind: "unsupportedHost" }
	| { kind: "notConnected"; host: HostKindView | null };

/** The problem of a context branch in one repository, or null when there is none or it is unknown. */
export type BranchHostProblemLookup = (
	repositoryId: string,
	branch: string,
) => BranchHostProblem | null;

function hostProblem({ host, status }: BranchPullRequestView): BranchHostProblem | null {
	switch (status.kind) {
		case "noRemote":
			return { kind: "noRemote" };
		case "unsupportedHost":
			return { kind: "unsupportedHost" };
		case "notConnected":
			return { kind: "notConnected", host };
		default:
			return null;
	}
}

export function branchHostProblemLookup(found: BranchPullRequestView[]): BranchHostProblemLookup {
	const problems = new Map<string, BranchHostProblem>();
	for (const branchPullRequest of found) {
		const problem = hostProblem(branchPullRequest);
		if (problem) {
			problems.set(lookupKey(branchPullRequest.repositoryId, branchPullRequest.branch), problem);
		}
	}
	return (repositoryId, branch) => problems.get(lookupKey(repositoryId, branch)) ?? null;
}

/** Whether the user is signed in to the Git host of at least one context branch. */
export function isAnyBranchHostConnected(found: BranchPullRequestView[]): boolean {
	return found.some(({ status }) => status.kind === "ok" || status.kind === "failed");
}
