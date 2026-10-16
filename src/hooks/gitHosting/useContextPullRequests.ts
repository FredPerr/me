import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { type BranchPullRequestView, commands } from "@/bindings";
import {
	branchHostProblemLookup,
	branchPullRequestLookup,
	isAnyBranchHostConnected,
} from "@/models/git-hosting/branchPullRequests";
import { GIT_HOSTING_QUERY_KEY } from "./gitHostingQueryKeys";

const NO_BRANCHES: BranchPullRequestView[] = [];

export function contextPullRequestsQueryKey(projectTag: string) {
	return [...GIT_HOSTING_QUERY_KEY, "contextPullRequests", projectTag] as const;
}

/** The pull request of each branch the project's contexts use, in one call for the whole board. */
export function useContextPullRequests(projectTag: string) {
	const query = useQuery({
		queryKey: contextPullRequestsQueryKey(projectTag),
		queryFn: () => commands.gitHostingContextPullRequests(projectTag),
	});
	const found = query.data ?? NO_BRANCHES;
	const lookup = useMemo(() => branchPullRequestLookup(found), [found]);
	const hostProblem = useMemo(() => branchHostProblemLookup(found), [found]);
	return { connected: isAnyBranchHostConnected(found), lookup, hostProblem };
}
