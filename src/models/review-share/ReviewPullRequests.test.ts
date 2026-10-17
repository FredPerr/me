import { describe, expect, it } from "vitest";
import type { HostedPullRequestView, RepositoryPullRequestsView } from "@/bindings";
import { reviewPullRequests } from "./ReviewPullRequests";

const API_ID = "api-id";
const API_NAME = "api";
const WEB_ID = "web-id";
const WEB_NAME = "web";
const SHARED_NUMBER = 7;
const OTHER_NUMBER = 8;

function buildPullRequest(number: number): HostedPullRequestView {
	return {
		number,
		title: "Add the login page",
		url: `https://github.com/acme/api/pull/${number}`,
		author: "octocat",
		headBranch: "feature/login",
		baseBranch: "main",
		isFork: false,
		draft: false,
		state: "open",
		updatedAt: "2024-05-01T12:00:00Z",
	};
}

function listedRepository(id: string, name: string, numbers: number[]): RepositoryPullRequestsView {
	return {
		repositoryId: id,
		repositoryName: name,
		host: "github",
		status: { kind: "ok", pullRequests: numbers.map(buildPullRequest), truncated: false },
	};
}

describe("reviewPullRequests", () => {
	it("tags each pull request with its repository", () => {
		const [pullRequest] = reviewPullRequests([listedRepository(API_ID, API_NAME, [SHARED_NUMBER])]);

		expect(pullRequest).toMatchObject({
			number: SHARED_NUMBER,
			repositoryName: API_NAME,
			host: "github",
		});
	});

	it("keeps pull requests with the same number in different repositories apart", () => {
		const pullRequests = reviewPullRequests([
			listedRepository(API_ID, API_NAME, [SHARED_NUMBER]),
			listedRepository(WEB_ID, WEB_NAME, [SHARED_NUMBER, OTHER_NUMBER]),
		]);

		const distinctIds = new Set(pullRequests.map((pullRequest) => pullRequest.id));
		expect(distinctIds.size).toBe(pullRequests.length);
	});

	it("leaves out repositories that were not listed", () => {
		const pullRequests = reviewPullRequests([
			{ repositoryId: API_ID, repositoryName: API_NAME, host: null, status: { kind: "noRemote" } },
			listedRepository(WEB_ID, WEB_NAME, [OTHER_NUMBER]),
		]);

		expect(pullRequests.map((pullRequest) => pullRequest.repositoryName)).toEqual([WEB_NAME]);
	});
});
