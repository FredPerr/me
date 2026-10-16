import { queryOptions, useQueries, useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { GIT_HOSTING_QUERY_KEY } from "./gitHostingQueryKeys";

export function projectPullRequestsQueryKey(projectTag: string) {
	return [...GIT_HOSTING_QUERY_KEY, "projectPullRequests", projectTag] as const;
}

function projectPullRequestsQuery(projectTag: string) {
	return queryOptions({
		queryKey: projectPullRequestsQueryKey(projectTag),
		queryFn: () => commands.gitHostingProjectPullRequests(projectTag),
	});
}

/** The open pull requests of each repository of the project, each with its own status. */
export function useProjectPullRequests(projectTag: string) {
	return useQuery(projectPullRequestsQuery(projectTag));
}

/** `useProjectPullRequests` for several projects, in the order of `projectTags`. */
export function useProjectsPullRequests(projectTags: string[]) {
	return useQueries({ queries: projectTags.map(projectPullRequestsQuery) });
}
