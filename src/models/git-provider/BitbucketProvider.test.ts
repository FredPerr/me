import { describe, expect, it } from "vitest";
import { parseBitbucketRepositoryRef } from "./BitbucketProvider";

const WORKSPACE = "fredperr";
const REPO_SLUG = "me";

describe("parseBitbucketRepositoryRef", () => {
	it("parses a normalized https bitbucket url", () => {
		const result = parseBitbucketRepositoryRef(`https://bitbucket.org/${WORKSPACE}/${REPO_SLUG}`);

		expect(result).toEqual({ owner: WORKSPACE, repo: REPO_SLUG });
	});

	it("strips a trailing .git suffix", () => {
		const result = parseBitbucketRepositoryRef(
			`https://bitbucket.org/${WORKSPACE}/${REPO_SLUG}.git`,
		);

		expect(result).toEqual({ owner: WORKSPACE, repo: REPO_SLUG });
	});

	it("parses an ssh bitbucket url", () => {
		const result = parseBitbucketRepositoryRef(`git@bitbucket.org:${WORKSPACE}/${REPO_SLUG}.git`);

		expect(result).toEqual({ owner: WORKSPACE, repo: REPO_SLUG });
	});

	it("ignores trailing path segments beyond workspace and repo", () => {
		const result = parseBitbucketRepositoryRef(
			`https://bitbucket.org/${WORKSPACE}/${REPO_SLUG}/src/main`,
		);

		expect(result).toEqual({ owner: WORKSPACE, repo: REPO_SLUG });
	});

	it("returns null for a non-bitbucket url", () => {
		const result = parseBitbucketRepositoryRef(`https://github.com/${WORKSPACE}/${REPO_SLUG}`);

		expect(result).toBeNull();
	});

	it("returns null when the repo segment is missing", () => {
		const result = parseBitbucketRepositoryRef(`https://bitbucket.org/${WORKSPACE}`);

		expect(result).toBeNull();
	});
});
