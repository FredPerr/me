import { describe, expect, it, vi } from "vitest";
import { resolvePath } from "./resolvePath";

const MOCK_HOME_DIR = "/Users/test-user/";
const PROJECT_BASE_PATH = "/Users/test-user/Projects/app";
const SUBDIR_NAME = "backend";
const RELATIVE_SUBDIR = `./${SUBDIR_NAME}`;
const HOME_RELATIVE_PATH = "~/Projects/my-app";
const ABSOLUTE_PATH = "/opt/projects/app";

vi.mock("@tauri-apps/api/path", () => ({
	homeDir: vi.fn().mockResolvedValue("/Users/test-user/"),
}));

describe("resolvePath", () => {
	describe("home directory resolution", () => {
		it("resolves tilde alone to the home directory", async () => {
			const path = "~";

			const result = await resolvePath(path, {});

			expect(result).toBe(MOCK_HOME_DIR);
		});

		it("resolves tilde-prefixed path to home directory concatenated with the rest", async () => {
			const path = HOME_RELATIVE_PATH;
			const expectedSuffix = HOME_RELATIVE_PATH.slice(2);

			const result = await resolvePath(path, {});

			expect(result).toBe(`${MOCK_HOME_DIR}${expectedSuffix}`);
		});
	});

	describe("relative path resolution with basePath", () => {
		it("resolves dot-slash prefixed path relative to basePath", async () => {
			const path = RELATIVE_SUBDIR;
			const options = { basePath: PROJECT_BASE_PATH };

			const result = await resolvePath(path, options);

			expect(result).toBe(`${PROJECT_BASE_PATH}/${SUBDIR_NAME}`);
		});

		it("resolves dot alone to basePath itself", async () => {
			const path = ".";
			const options = { basePath: PROJECT_BASE_PATH };

			const result = await resolvePath(path, options);

			expect(result).toBe(PROJECT_BASE_PATH);
		});

		it("resolves bare relative name against basePath", async () => {
			const path = SUBDIR_NAME;
			const options = { basePath: PROJECT_BASE_PATH };

			const result = await resolvePath(path, options);

			expect(result).toBe(`${PROJECT_BASE_PATH}/${SUBDIR_NAME}`);
		});

		it("returns relative path unchanged when no basePath is provided", async () => {
			const path = RELATIVE_SUBDIR;

			const result = await resolvePath(path, {});

			expect(result).toBe(RELATIVE_SUBDIR);
		});
	});

	describe("absolute paths", () => {
		it("returns absolute path unchanged", async () => {
			const path = ABSOLUTE_PATH;

			const result = await resolvePath(path, {});

			expect(result).toBe(ABSOLUTE_PATH);
		});
	});

	describe("isFolder option", () => {
		it("appends trailing slash when path does not end with one", async () => {
			const path = ABSOLUTE_PATH;
			const options = { isFolder: true };

			const result = await resolvePath(path, options);

			expect(result).toBe(`${ABSOLUTE_PATH}/`);
		});

		it("does not double trailing slash when path already ends with one", async () => {
			const path = `${ABSOLUTE_PATH}/`;
			const options = { isFolder: true };

			const result = await resolvePath(path, options);

			expect(result).toBe(`${ABSOLUTE_PATH}/`);
		});
	});
});
