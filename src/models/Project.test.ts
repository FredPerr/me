import { describe, expect, it } from "vitest";
import { Project, Repository } from "./Project";

const PROJECT_NAME = "My Project";
const PROJECT_TAG = "my-project";
const PROJECT_PATH = "/Users/me/projects/my-project";

function buildProject(repositories: Repository[]): Project {
	return new Project(PROJECT_NAME, PROJECT_TAG, PROJECT_PATH, repositories, []);
}

function buildRepository(id: string, name: string, relPath: string): Repository {
	return new Repository(id, name, relPath);
}

describe("Project", () => {
	describe("effectiveRepositories", () => {
		it("returns the configured repositories when there are some", () => {
			const configured = [buildRepository("repo-1", "api", "./api")];
			const project = buildProject(configured);

			const result = project.effectiveRepositories();

			expect(result).toBe(configured);
		});

		it("returns a synthetic root repository for a single-repo project", () => {
			const project = buildProject([]);

			const result = project.effectiveRepositories();

			expect(result).toHaveLength(1);
			expect(result[0].id).toBe(PROJECT_TAG);
			expect(result[0].name).toBe(PROJECT_NAME);
			expect(result[0].relPath).toBe(".");
		});
	});

	describe("findEffectiveRepository", () => {
		it("finds a configured repository by id", () => {
			const project = buildProject([buildRepository("repo-1", "api", "./api")]);

			const result = project.findEffectiveRepository("repo-1");

			expect(result?.name).toBe("api");
		});

		it("resolves the synthetic root repository by the project tag", () => {
			const project = buildProject([]);

			const result = project.findEffectiveRepository(PROJECT_TAG);

			expect(result?.relPath).toBe(".");
		});

		it("returns undefined for an unknown id", () => {
			const project = buildProject([buildRepository("repo-1", "api", "./api")]);

			const result = project.findEffectiveRepository("missing");

			expect(result).toBeUndefined();
		});
	});
});
