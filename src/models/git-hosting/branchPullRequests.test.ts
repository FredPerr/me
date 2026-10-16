import { describe, expect, it } from "vitest";
import type {
	BranchPullRequestStatusView,
	BranchPullRequestView,
	HostedPullRequestView,
} from "@/bindings";
import {
	branchHostProblemLookup,
	branchPullRequestLookup,
	isAnyBranchHostConnected,
} from "./branchPullRequests";

const API_ID = "api-id";
const WEB_ID = "web-id";
const LOGIN_BRANCH = "feature/login";
const SEARCH_BRANCH = "feature/search";
const PULL_REQUEST_NUMBER = 7;

function buildPullRequest(overrides?: Partial<HostedPullRequestView>): HostedPullRequestView {
	return {
		number: PULL_REQUEST_NUMBER,
		title: "Add the login page",
		url: "https://github.com/acme/api/pull/7",
		author: "octocat",
		headBranch: LOGIN_BRANCH,
		baseBranch: "main",
		draft: false,
		state: "open",
		updatedAt: "2024-05-01T12:00:00Z",
		...overrides,
	};
}

function buildBranch(
	status: BranchPullRequestStatusView,
	overrides?: Partial<BranchPullRequestView>,
): BranchPullRequestView {
	return { repositoryId: API_ID, branch: LOGIN_BRANCH, host: "github", status, ...overrides };
}

function found(pullRequest: HostedPullRequestView | null): BranchPullRequestStatusView {
	return { kind: "ok", pullRequest };
}

const FAILED: BranchPullRequestStatusView = {
	kind: "failed",
	error: {
		kind: "network",
		message: "The Git host could not be reached.",
		retryAfterSeconds: null,
		status: null,
	},
};

describe("branchPullRequestLookup", () => {
	it("returns the pull request of a repository branch", () => {
		const lookup = branchPullRequestLookup([buildBranch(found(buildPullRequest()))]);

		expect(lookup(API_ID, LOGIN_BRANCH)).toEqual(buildPullRequest());
	});

	it("tells the same branch of two repositories apart", () => {
		const webPullRequest = buildPullRequest({ state: "merged" });
		const lookup = branchPullRequestLookup([
			buildBranch(found(buildPullRequest())),
			buildBranch(found(webPullRequest), { repositoryId: WEB_ID }),
		]);

		expect(lookup(WEB_ID, LOGIN_BRANCH)).toEqual(webPullRequest);
	});

	it("returns null for a branch without pull request", () => {
		const lookup = branchPullRequestLookup([buildBranch(found(null))]);

		expect(lookup(API_ID, LOGIN_BRANCH)).toBeNull();
	});

	it("returns null for a branch whose lookup failed", () => {
		const lookup = branchPullRequestLookup([buildBranch(FAILED)]);

		expect(lookup(API_ID, LOGIN_BRANCH)).toBeNull();
	});

	it("returns null for a branch it was not given", () => {
		const lookup = branchPullRequestLookup([buildBranch(found(buildPullRequest()))]);

		expect(lookup(API_ID, SEARCH_BRANCH)).toBeNull();
	});
});

describe("isAnyBranchHostConnected", () => {
	it("is true when a branch was looked up", () => {
		expect(
			isAnyBranchHostConnected([
				buildBranch({ kind: "notConnected" }),
				buildBranch(found(null), { branch: SEARCH_BRANCH }),
			]),
		).toBe(true);
	});

	it("is true when a lookup failed on a signed-in host", () => {
		expect(isAnyBranchHostConnected([buildBranch(FAILED)])).toBe(true);
	});

	it("is false when no branch is on a signed-in host", () => {
		expect(
			isAnyBranchHostConnected([
				buildBranch({ kind: "notConnected" }),
				buildBranch({ kind: "noRemote" }, { host: null, branch: SEARCH_BRANCH }),
			]),
		).toBe(false);
	});
});

describe("branchHostProblemLookup", () => {
	it("says a branch of a repository without remote has no remote", () => {
		const lookup = branchHostProblemLookup([buildBranch({ kind: "noRemote" }, { host: null })]);

		expect(lookup(API_ID, LOGIN_BRANCH)).toEqual({ kind: "noRemote" });
	});

	it("says a branch on another Git host is unsupported", () => {
		const lookup = branchHostProblemLookup([
			buildBranch({ kind: "unsupportedHost" }, { host: null }),
		]);

		expect(lookup(API_ID, LOGIN_BRANCH)).toEqual({ kind: "unsupportedHost" });
	});

	it("names the Git host a branch is not connected to", () => {
		const lookup = branchHostProblemLookup([
			buildBranch({ kind: "notConnected" }, { host: "bitbucket" }),
		]);

		expect(lookup(API_ID, LOGIN_BRANCH)).toEqual({ kind: "notConnected", host: "bitbucket" });
	});

	it("returns null for a branch that was looked up", () => {
		const lookup = branchHostProblemLookup([buildBranch(found(null)), buildBranch(FAILED)]);

		expect(lookup(API_ID, LOGIN_BRANCH)).toBeNull();
	});
});
