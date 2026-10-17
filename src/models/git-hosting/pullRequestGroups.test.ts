import { describe, expect, it } from "vitest";
import type {
	HostedPullRequestView,
	PullRequestsStatusView,
	RepositoryPullRequestsView,
} from "@/bindings";
import {
	groupByHeadBranch,
	type PullRequestGroup,
	selectionForGroup,
	suggestedName,
} from "./pullRequestGroups";

const API = "api";
const WEB = "web";
const LOGIN_BRANCH = "feature/login";
const SEARCH_BRANCH = "feature/search";
const MAIN_BRANCH = "main";
const LOGIN_TITLE = "Add the login page";

function buildPullRequest(overrides?: Partial<HostedPullRequestView>): HostedPullRequestView {
	return {
		number: 12,
		title: LOGIN_TITLE,
		url: "https://github.com/acme/api/pull/12",
		author: "octocat",
		headBranch: LOGIN_BRANCH,
		baseBranch: MAIN_BRANCH,
		isFork: false,
		draft: false,
		state: "open",
		updatedAt: "2024-05-01T12:00:00Z",
		...overrides,
	};
}

function listed(pullRequests: HostedPullRequestView[]): PullRequestsStatusView {
	return { kind: "ok", pullRequests, truncated: false };
}

function buildRepository(
	name: string,
	pullRequests: HostedPullRequestView[],
	overrides?: Partial<RepositoryPullRequestsView>,
): RepositoryPullRequestsView {
	return {
		repositoryId: `${name}-id`,
		repositoryName: name,
		host: "github",
		status: listed(pullRequests),
		...overrides,
	};
}

function groupWith(headBranch: string): PullRequestGroup {
	const repositories = [
		buildRepository(API, [buildPullRequest({ number: 12, headBranch })]),
		buildRepository(WEB, [buildPullRequest({ number: 34, headBranch })]),
	];
	const [group] = groupByHeadBranch(repositories);
	return group;
}

describe("groupByHeadBranch", () => {
	it("merges pull requests sharing a head branch across repositories into one group", () => {
		const repositories = [
			buildRepository(API, [buildPullRequest({ number: 12, headBranch: LOGIN_BRANCH })]),
			buildRepository(WEB, [buildPullRequest({ number: 34, headBranch: LOGIN_BRANCH })]),
		];

		const groups = groupByHeadBranch(repositories);

		expect(groups).toHaveLength(1);
		expect(groups[0].headBranch).toBe(LOGIN_BRANCH);
		expect(groups[0].items.map((item) => item.repositoryId)).toEqual([`${API}-id`, `${WEB}-id`]);
	});

	it("sorts groups by head branch", () => {
		const repositories = [
			buildRepository(API, [
				buildPullRequest({ number: 12, headBranch: SEARCH_BRANCH }),
				buildPullRequest({ number: 13, headBranch: LOGIN_BRANCH }),
			]),
		];

		const groups = groupByHeadBranch(repositories);

		expect(groups.map((group) => group.headBranch)).toEqual([LOGIN_BRANCH, SEARCH_BRANCH]);
	});

	it("includes fork pull requests but flags them", () => {
		const repositories = [
			buildRepository(API, [buildPullRequest({ headBranch: LOGIN_BRANCH, isFork: true })]),
		];

		const groups = groupByHeadBranch(repositories);

		expect(groups[0].items[0].isFork).toBe(true);
	});

	it("carries each item's host from its repository", () => {
		const repositories = [buildRepository(API, [buildPullRequest()], { host: "bitbucket" })];

		const groups = groupByHeadBranch(repositories);

		expect(groups[0].items[0].host).toBe("bitbucket");
	});
});

describe("selectionForGroup", () => {
	it("picks the head branch in every repository with a non-fork pull request", () => {
		const selection = selectionForGroup(groupWith(LOGIN_BRANCH));

		expect(selection).toEqual({ [`${API}-id`]: LOGIN_BRANCH, [`${WEB}-id`]: LOGIN_BRANCH });
	});

	it("skips fork pull requests", () => {
		const repositories = [
			buildRepository(API, [buildPullRequest({ headBranch: LOGIN_BRANCH })]),
			buildRepository(WEB, [buildPullRequest({ headBranch: LOGIN_BRANCH, isFork: true })]),
		];
		const [group] = groupByHeadBranch(repositories);

		const selection = selectionForGroup(group);

		expect(selection).toEqual({ [`${API}-id`]: LOGIN_BRANCH });
	});
});

describe("suggestedName", () => {
	it("suggests the title of a single pull request", () => {
		expect(suggestedName([buildPullRequest({ title: LOGIN_TITLE })])).toBe(LOGIN_TITLE);
	});

	it("suggests the shared head branch for several pull requests", () => {
		const selected = [
			buildPullRequest({ number: 12, title: "API login", headBranch: LOGIN_BRANCH }),
			buildPullRequest({ number: 34, title: "Web login", headBranch: LOGIN_BRANCH }),
		];

		expect(suggestedName(selected)).toBe(LOGIN_BRANCH);
	});

	it("suggests nothing when the pull requests do not share a head branch", () => {
		const selected = [
			buildPullRequest({ number: 12, headBranch: LOGIN_BRANCH }),
			buildPullRequest({ number: 34, headBranch: SEARCH_BRANCH }),
		];

		expect(suggestedName(selected)).toBe("");
	});

	it("suggests nothing when nothing is selected", () => {
		expect(suggestedName([])).toBe("");
	});
});
