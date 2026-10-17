import { describe, expect, it } from "vitest";
import type { HostedPullRequestView } from "@/bindings";
import { pullRequestIconState } from "./pullRequestIconState";

function buildPullRequest(overrides?: Partial<HostedPullRequestView>): HostedPullRequestView {
	return {
		number: 7,
		title: "Add the login page",
		url: "https://github.com/acme/api/pull/7",
		author: "octocat",
		headBranch: "feature/login",
		baseBranch: "main",
		isFork: false,
		draft: false,
		state: "open",
		updatedAt: "2024-05-01T12:00:00Z",
		...overrides,
	};
}

describe("pullRequestIconState", () => {
	it("shows an open pull request as open", () => {
		expect(pullRequestIconState(buildPullRequest())).toBe("open");
	});

	it("shows an open draft as a draft", () => {
		expect(pullRequestIconState(buildPullRequest({ draft: true }))).toBe("draft");
	});

	it("shows a merged pull request as merged", () => {
		expect(pullRequestIconState(buildPullRequest({ state: "merged" }))).toBe("merged");
	});

	it("shows a closed pull request as closed", () => {
		expect(pullRequestIconState(buildPullRequest({ state: "closed" }))).toBe("closed");
	});
});
