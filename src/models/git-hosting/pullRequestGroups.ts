import type { HostedPullRequestView, HostKindView, RepositoryPullRequestsView } from "@/bindings";
import { listedPullRequests } from "./repositoryPullRequests";

/** One repository's pull request within a head-branch group, with the data needed to pick it. */
export type PullRequestGroupItem = {
	repositoryId: string;
	repositoryName: string;
	host: HostKindView | null;
	pullRequest: HostedPullRequestView;
	isFork: boolean;
};

/** The open pull requests across repositories that share a head branch. */
export type PullRequestGroup = {
	headBranch: string;
	items: PullRequestGroupItem[];
};

/** A repository's selected pull request, kept by head branch. */
export type RepositorySelection = Record<string, string>;

/**
 * Group the project's open pull requests by head branch across repositories.
 * Fork pull requests are included but flagged, so the view can disable them.
 */
export function groupByHeadBranch(repositories: RepositoryPullRequestsView[]): PullRequestGroup[] {
	const groupsByHeadBranch = new Map<string, PullRequestGroupItem[]>();
	for (const repository of repositories) {
		for (const pullRequest of listedPullRequests(repository)) {
			const item: PullRequestGroupItem = {
				repositoryId: repository.repositoryId,
				repositoryName: repository.repositoryName,
				host: repository.host,
				pullRequest,
				isFork: pullRequest.isFork,
			};
			const existing = groupsByHeadBranch.get(pullRequest.headBranch) ?? [];
			existing.push(item);
			groupsByHeadBranch.set(pullRequest.headBranch, existing);
		}
	}
	return [...groupsByHeadBranch.entries()]
		.map(([headBranch, items]) => ({ headBranch, items }))
		.sort((left, right) => left.headBranch.localeCompare(right.headBranch));
}

/**
 * Picking a group selects its head branch in every repository that has a
 * non-fork pull request for it. Fork pull requests are skipped: they are
 * shown disabled and cannot be checked out.
 */
export function selectionForGroup(group: PullRequestGroup): RepositorySelection {
	const selection: RepositorySelection = {};
	for (const item of group.items) {
		if (item.isFork) continue;
		selection[item.repositoryId] = item.pullRequest.headBranch;
	}
	return selection;
}

/**
 * Derive the suggested context name from the selected pull requests:
 * a single pull request → its title; several sharing a head branch → the
 * head branch (prefix included); otherwise an empty string.
 */
export function suggestedName(selectedPullRequests: HostedPullRequestView[]): string {
	if (selectedPullRequests.length === 0) return "";
	if (selectedPullRequests.length === 1) return selectedPullRequests[0].title;
	const headBranches = new Set(selectedPullRequests.map((pullRequest) => pullRequest.headBranch));
	return headBranches.size === 1 ? [...headBranches][0] : "";
}
