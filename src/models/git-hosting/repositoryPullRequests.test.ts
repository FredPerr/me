import { describe, expect, it } from "vitest";
import type {
	HostedPullRequestView,
	PullRequestsStatusView,
	RepositoryPullRequestsView,
} from "@/bindings";
import {
	isEveryHostNotConnected,
	labeledByRepositoryName,
	listedPullRequests,
	pullRequestsNotices,
} from "./repositoryPullRequests";

const API = "api";
const WEB = "web";
const DOCS = "docs";
const FAILURE_MESSAGE = "The Git host rejected the credentials.";
const PULL_REQUEST_NUMBER = 7;

function buildPullRequest(overrides?: Partial<HostedPullRequestView>): HostedPullRequestView {
	return {
		number: PULL_REQUEST_NUMBER,
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

function listed(truncated = false): PullRequestsStatusView {
	return { kind: "ok", pullRequests: [buildPullRequest()], truncated };
}

function failed(): PullRequestsStatusView {
	return {
		kind: "failed",
		error: {
			kind: "unauthorized",
			message: FAILURE_MESSAGE,
			retryAfterSeconds: null,
			status: null,
		},
	};
}

function buildRepository(
	name: string,
	status: PullRequestsStatusView,
	overrides?: Partial<RepositoryPullRequestsView>,
): RepositoryPullRequestsView {
	return { repositoryId: `${name}-id`, repositoryName: name, host: "github", status, ...overrides };
}

function noticesOf(repositories: RepositoryPullRequestsView[]) {
	return pullRequestsNotices(labeledByRepositoryName(repositories));
}

describe("listedPullRequests", () => {
	it("returns the pull requests of a listed repository", () => {
		expect(listedPullRequests(buildRepository(API, listed()))).toEqual([buildPullRequest()]);
	});

	it("returns nothing for a repository that was not listed", () => {
		expect(listedPullRequests(buildRepository(API, failed()))).toEqual([]);
	});
});

describe("pullRequestsNotices", () => {
	it("groups the repositories not connected by Git host", () => {
		const notices = noticesOf([
			buildRepository(API, { kind: "notConnected" }),
			buildRepository(WEB, { kind: "notConnected" }, { host: "bitbucket" }),
			buildRepository(DOCS, { kind: "notConnected" }),
		]);

		expect(notices.notConnected).toEqual([
			{ host: "github", labels: [API, DOCS] },
			{ host: "bitbucket", labels: [WEB] },
		]);
	});

	it("names each failed repository with its message", () => {
		const notices = noticesOf([buildRepository(API, failed()), buildRepository(WEB, listed())]);

		expect(notices.failures).toEqual([{ label: API, message: FAILURE_MESSAGE }]);
	});

	it("names the repositories whose list was cut short", () => {
		const notices = noticesOf([buildRepository(API, listed(true)), buildRepository(WEB, listed())]);

		expect(notices.truncatedLabels).toEqual([API]);
	});

	it("has nothing to say about repositories without a supported remote", () => {
		const notices = noticesOf([
			buildRepository(API, { kind: "noRemote" }, { host: null }),
			buildRepository(WEB, { kind: "unsupportedHost" }, { host: null }),
		]);

		expect(notices).toEqual({ notConnected: [], failures: [], truncatedLabels: [] });
	});
});

describe("isEveryHostNotConnected", () => {
	it("is true when every hosted repository is not connected", () => {
		const repositories = [
			buildRepository(API, { kind: "notConnected" }),
			buildRepository(WEB, { kind: "noRemote" }, { host: null }),
		];

		expect(isEveryHostNotConnected(repositories)).toBe(true);
	});

	it("is false when one hosted repository is listed", () => {
		const repositories = [
			buildRepository(API, { kind: "notConnected" }),
			buildRepository(WEB, listed(), { host: "bitbucket" }),
		];

		expect(isEveryHostNotConnected(repositories)).toBe(false);
	});

	it("is false without hosted repositories", () => {
		expect(
			isEveryHostNotConnected([buildRepository(API, { kind: "noRemote" }, { host: null })]),
		).toBe(false);
	});
});
