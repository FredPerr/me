import { useMemo } from "react";
import { useProjectPullRequests } from "@/hooks/useProjectPullRequests";
import type { ProviderPullRequest } from "@/models/git-provider/GitProvider";
import type { Project } from "@/models/Project";

/** A project PR flattened for the review-share modal, tagged with its repo. */
export type ReviewPullRequest = ProviderPullRequest & {
	repositoryName: string;
};

type UseProjectReviewPullRequestsResult = {
	connected: boolean;
	loading: boolean;
	pullRequests: ReviewPullRequest[];
	error: string | null;
};

/**
 * Loads a single project's open pull requests as a flat, repository-tagged
 * list suitable for building a review message. Reuses the grouped
 * project-pull-requests hook so remote resolution stays in one place.
 */
export function useProjectReviewPullRequests(project: Project): UseProjectReviewPullRequestsResult {
	const projects = useMemo(() => [project], [project]);
	const { connected, loading, groups, error } = useProjectPullRequests(projects);

	const pullRequests = useMemo(
		() =>
			groups.flatMap((group) =>
				group.pullRequests.map((pullRequest) => ({
					...pullRequest,
					repositoryName: group.repository.name,
				})),
			),
		[groups],
	);

	return { connected, loading, pullRequests, error };
}
