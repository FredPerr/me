import type { HostedPullRequestView } from "@/bindings";

/** What a pull request's icon shows: a draft only while open. */
export type PullRequestIconState = "draft" | "open" | "merged" | "closed";

export function pullRequestIconState(pullRequest: HostedPullRequestView): PullRequestIconState {
	if (pullRequest.state === "open" && pullRequest.draft) return "draft";
	return pullRequest.state;
}
