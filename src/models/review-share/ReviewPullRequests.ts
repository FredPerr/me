import type { HostedPullRequestView, HostKindView, RepositoryPullRequestsView } from "@/bindings";
import { listedPullRequests } from "@/models/git-hosting/repositoryPullRequests";

/** A project pull request offered for the review message, tagged with its repository. */
export type ReviewPullRequest = HostedPullRequestView & {
	/** Unique across the project's repositories, whose pull request numbers can collide. */
	id: string;
	repositoryName: string;
	host: HostKindView | null;
};

/** The listed pull requests of every repository, flattened in repository order. */
export function reviewPullRequests(
	repositories: RepositoryPullRequestsView[],
): ReviewPullRequest[] {
	return repositories.flatMap((repository) =>
		listedPullRequests(repository).map((pullRequest) => ({
			...pullRequest,
			id: `${repository.repositoryId}#${pullRequest.number}`,
			repositoryName: repository.repositoryName,
			host: repository.host,
		})),
	);
}
