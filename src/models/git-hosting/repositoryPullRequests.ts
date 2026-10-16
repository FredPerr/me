import type { HostedPullRequestView, HostKindView, RepositoryPullRequestsView } from "@/bindings";

/** A repository's pull requests with the name to show it under. */
export type LabeledRepositoryPullRequests = {
	label: string;
	repository: RepositoryPullRequestsView;
};

/** What the user should know about the repositories whose pull requests are not all shown. */
export type PullRequestsNotices = {
	notConnected: { host: HostKindView; labels: string[] }[];
	failures: { label: string; message: string }[];
	truncatedLabels: string[];
};

export function listedPullRequests(
	repository: RepositoryPullRequestsView,
): HostedPullRequestView[] {
	return repository.status.kind === "ok" ? repository.status.pullRequests : [];
}

export function labeledByRepositoryName(
	repositories: RepositoryPullRequestsView[],
): LabeledRepositoryPullRequests[] {
	return repositories.map((repository) => ({ label: repository.repositoryName, repository }));
}

/** Repositories without remote or on another Git host are left out, as there is nothing to do. */
export function pullRequestsNotices(entries: LabeledRepositoryPullRequests[]): PullRequestsNotices {
	const notConnectedLabels = new Map<HostKindView, string[]>();
	const failures: PullRequestsNotices["failures"] = [];
	const truncatedLabels: string[] = [];
	for (const { label, repository } of entries) {
		const { status, host } = repository;
		if (status.kind === "notConnected" && host) {
			notConnectedLabels.set(host, [...(notConnectedLabels.get(host) ?? []), label]);
		} else if (status.kind === "failed") {
			failures.push({ label, message: status.error.message });
		} else if (status.kind === "ok" && status.truncated) {
			truncatedLabels.push(label);
		}
	}
	return {
		notConnected: [...notConnectedLabels].map(([host, labels]) => ({ host, labels })),
		failures,
		truncatedLabels,
	};
}

/** Whether there are Git host repositories and the user is signed in to none of their hosts. */
export function isEveryHostNotConnected(repositories: RepositoryPullRequestsView[]): boolean {
	const hosted = repositories.filter((repository) => repository.host !== null);
	return (
		hosted.length > 0 && hosted.every((repository) => repository.status.kind === "notConnected")
	);
}
