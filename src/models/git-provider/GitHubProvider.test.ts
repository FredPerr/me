import { describe, expect, it } from "vitest";
import { parseGitHubRepositoryRef } from "./GitHubProvider";

const OWNER = "FredPerr";
const REPO = "me";

describe("parseGitHubRepositoryRef", () => {
	it("parses a normalized https github url", () => {
		const result = parseGitHubRepositoryRef(`https://github.com/${OWNER}/${REPO}`);

		expect(result).toEqual({ owner: OWNER, repo: REPO });
	});

	it("strips a trailing .git suffix", () => {
		const result = parseGitHubRepositoryRef(`https://github.com/${OWNER}/${REPO}.git`);

		expect(result).toEqual({ owner: OWNER, repo: REPO });
	});

	it("parses an ssh github url", () => {
		const result = parseGitHubRepositoryRef(`git@github.com:${OWNER}/${REPO}.git`);

		expect(result).toEqual({ owner: OWNER, repo: REPO });
	});

	it("ignores trailing path segments beyond owner and repo", () => {
		const result = parseGitHubRepositoryRef(`https://github.com/${OWNER}/${REPO}/tree/main`);

		expect(result).toEqual({ owner: OWNER, repo: REPO });
	});

	it("returns null for a non-github url", () => {
		const result = parseGitHubRepositoryRef(`https://gitlab.com/${OWNER}/${REPO}`);

		expect(result).toBeNull();
	});

	it("returns null when the repo segment is missing", () => {
		const result = parseGitHubRepositoryRef(`https://github.com/${OWNER}`);

		expect(result).toBeNull();
	});
});
