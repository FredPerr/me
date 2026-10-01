import { describe, expect, it } from "vitest";
import { ProviderKind } from "@/domain/work-tracking/ProviderKind";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";
import { Context, ContextBranch, Project, type ProjectData, Repository } from "./Project";

const PROJECT_NAME = "My Project";
const PROJECT_TAG = "my-project";
const PROJECT_PATH = "/Users/me/projects/my-project";

function buildProject(repositories: Repository[]): Project {
	return new Project(PROJECT_NAME, PROJECT_TAG, PROJECT_PATH, repositories, []);
}

function buildRepository(id: string, name: string, relPath: string): Repository {
	return new Repository(id, name, relPath);
}
function buildContext(name: string, repositoryIds: string[], isDefault = false): Context {
	return new Context(
		`context-${name}`,
		name,
		repositoryIds.map((repositoryId) => new ContextBranch(repositoryId, "feature")),
		isDefault,
	);
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

	describe("Context.getWorktreePath", () => {
		it("returns the project path for the root repository of the default context", () => {
			const project = buildProject([]);
			const context = buildContext("main", [PROJECT_TAG], true);
			const [rootRepository] = project.effectiveRepositories();

			const result = context.getWorktreePath(PROJECT_PATH, rootRepository);

			expect(result).toBe(PROJECT_PATH);
		});
	});

	describe("resolveContextIdePathCandidates", () => {
		const contextFolderPath = `${PROJECT_PATH}/.worktrees/feature-x`;

		it("starts with the repository worktree when the context has a single repository", () => {
			const project = buildProject([buildRepository("repo-1", "api", "./api")]);
			const context = buildContext("feature-x", ["repo-1"]);

			const result = project.resolveContextIdePathCandidates(context);

			expect(result).toEqual([`${contextFolderPath}/api`, contextFolderPath, PROJECT_PATH]);
		});

		it("starts with the context folder when the context has multiple repositories", () => {
			const project = buildProject([
				buildRepository("repo-1", "api", "./api"),
				buildRepository("repo-2", "web", "./web"),
			]);
			const context = buildContext("feature-x", ["repo-1", "repo-2"]);

			const result = project.resolveContextIdePathCandidates(context);

			expect(result).toEqual([contextFolderPath, PROJECT_PATH]);
		});

		it("starts with the context folder when the context has no repositories", () => {
			const project = buildProject([buildRepository("repo-1", "api", "./api")]);
			const context = buildContext("feature-x", []);

			const result = project.resolveContextIdePathCandidates(context);

			expect(result).toEqual([contextFolderPath, PROJECT_PATH]);
		});

		it("ignores branches pointing to unknown repositories", () => {
			const project = buildProject([buildRepository("repo-1", "api", "./api")]);
			const context = buildContext("feature-x", ["missing"]);

			const result = project.resolveContextIdePathCandidates(context);

			expect(result).toEqual([contextFolderPath, PROJECT_PATH]);
		});

		it("uses the project name for the worktree of a project without configured repositories", () => {
			const project = buildProject([]);
			const context = buildContext("feature-x", [PROJECT_TAG]);

			const result = project.resolveContextIdePathCandidates(context);

			expect(result).toEqual([
				`${contextFolderPath}/${PROJECT_NAME}`,
				contextFolderPath,
				PROJECT_PATH,
			]);
		});

		it("uses the repository folder in the project for the default context", () => {
			const project = buildProject([buildRepository("repo-1", "api", "./api")]);
			const context = buildContext("main", ["repo-1"], true);

			const result = project.resolveContextIdePathCandidates(context);

			expect(result[0]).toBe(`${PROJECT_PATH}/api`);
		});

		it("starts with the project path for the default context of a root repository", () => {
			const project = buildProject([]);
			const context = buildContext("main", [PROJECT_TAG], true);

			const result = project.resolveContextIdePathCandidates(context);

			expect(result).toEqual([PROJECT_PATH, `${PROJECT_PATH}/.worktrees/main`]);
		});

		it("normalizes a trailing slash in the project path", () => {
			const project = new Project(
				PROJECT_NAME,
				PROJECT_TAG,
				`${PROJECT_PATH}/`,
				[buildRepository("repo-1", "api", "./api")],
				[],
			);
			const context = buildContext("feature-x", ["repo-1"]);

			const result = project.resolveContextIdePathCandidates(context);

			expect(result).toEqual([`${contextFolderPath}/api`, contextFolderPath, PROJECT_PATH]);
		});
	});

	describe("remoteProjectLinks", () => {
		function buildLinks(): RemoteProjectLinks {
			return RemoteProjectLinks.empty().add(
				RemoteProjectLink.create(
					{
						providerKind: ProviderKind.Teamwork,
						connectionId: "teamwork:acme.teamwork.com",
						remoteProjectId: "42",
						remoteProjectName: "Website",
						remoteProjectUrl: "https://acme.teamwork.com/app/projects/42",
					},
					new Date("2024-01-15T10:00:00.000Z"),
				),
			);
		}

		function buildLinkedProject(): Project {
			return new Project(
				PROJECT_NAME,
				PROJECT_TAG,
				PROJECT_PATH,
				[buildRepository("repo-1", "api", "./api")],
				[buildContext("main", ["repo-1"], true)],
				"rocket",
				["node_modules"],
				buildLinks(),
			);
		}

		it("loads a project file without links as having no links (AC18)", () => {
			const data: ProjectData = {
				name: PROJECT_NAME,
				tag: PROJECT_TAG,
				path: PROJECT_PATH,
				repositories: [],
				contexts: [],
			};

			const project = Project.fromJSON(data);

			expect(project.remoteProjectLinks.toArray()).toEqual([]);
			expect(project.toJSON().remoteProjectLinks).toEqual([]);
		});

		it("round-trips the links through toJSON and fromJSON", () => {
			const project = buildLinkedProject();

			const restored = Project.fromJSON(project.toJSON());

			expect(restored.toJSON()).toEqual(project.toJSON());
			expect(restored.remoteProjectLinks.toJSON()).toEqual(buildLinks().toJSON());
		});

		it("keeps the links when adding or removing a context", () => {
			const project = buildLinkedProject();
			const added = project.addContext(buildContext("feature-x", ["repo-1"]));

			const removed = added.removeContext("context-feature-x");

			expect(added.remoteProjectLinks).toBe(project.remoteProjectLinks);
			expect(added.contexts).toHaveLength(2);
			expect(removed.remoteProjectLinks).toBe(project.remoteProjectLinks);
			expect(removed.toJSON()).toEqual(project.toJSON());
		});

		it("replaces only the links with withRemoteProjectLinks", () => {
			const project = buildLinkedProject();

			const unlinked = project.withRemoteProjectLinks(RemoteProjectLinks.empty());

			expect(unlinked.remoteProjectLinks.toArray()).toEqual([]);
			expect({ ...unlinked.toJSON(), remoteProjectLinks: undefined }).toEqual({
				...project.toJSON(),
				remoteProjectLinks: undefined,
			});
		});
	});

	describe("resolveRootIdePathCandidates", () => {
		it("starts with the repository folder when the project has a single repository", async () => {
			const project = buildProject([buildRepository("repo-1", "api", "./api")]);

			const result = await project.resolveRootIdePathCandidates();

			expect(result).toEqual([`${PROJECT_PATH}/api`, PROJECT_PATH]);
		});

		it("returns the project path when the project has multiple repositories", async () => {
			const project = buildProject([
				buildRepository("repo-1", "api", "./api"),
				buildRepository("repo-2", "web", "./web"),
			]);

			const result = await project.resolveRootIdePathCandidates();

			expect(result).toEqual([PROJECT_PATH]);
		});

		it("returns the project path when the project has no configured repositories", async () => {
			const project = buildProject([]);

			const result = await project.resolveRootIdePathCandidates();

			expect(result).toEqual([PROJECT_PATH]);
		});
	});
});
