import { describe, expect, it } from "vitest";
import { ProviderKind } from "@/domain/work-tracking/ProviderKind";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";
import { BranchNamingPolicy, DEFAULT_BRANCH_PREFIXES } from "./BranchNaming";
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

	describe("Context.withExpanded", () => {
		it("defaults to collapsed when constructed without the flag", () => {
			const context = buildContext("feature-x", [PROJECT_TAG]);

			expect(context.expanded).toBe(false);
		});

		it("returns a new context with the expanded flag set", () => {
			const context = buildContext("feature-x", [PROJECT_TAG]);

			const result = context.withExpanded(true);

			expect(result.expanded).toBe(true);
			expect(context.expanded).toBe(false);
		});

		it("round-trips the expanded flag through toJSON and fromJSON", () => {
			const context = buildContext("feature-x", [PROJECT_TAG]).withExpanded(true);

			const restored = Context.fromJSON(context.toJSON());

			expect(restored.expanded).toBe(true);
		});

		it("defaults to collapsed when deserializing data saved without the flag", () => {
			const context = buildContext("feature-x", [PROJECT_TAG]);
			const { expanded: _omitted, ...dataWithoutExpanded } = context.toJSON();

			const restored = Context.fromJSON(dataWithoutExpanded);

			expect(restored.expanded).toBe(false);
		});
	});

	describe("Context.isStatic", () => {
		it("defaults to non-static for a regular context", () => {
			const context = buildContext("feature-x", [PROJECT_TAG]);

			expect(context.isStatic).toBe(false);
			expect(context.isEffectivelyStatic).toBe(false);
		});

		it("treats the default context as effectively static even when the flag is off", () => {
			const context = buildContext("default", [PROJECT_TAG], true);

			expect(context.isStatic).toBe(false);
			expect(context.isEffectivelyStatic).toBe(true);
		});

		it("returns a new context with the static flag set", () => {
			const context = buildContext("staging", [PROJECT_TAG]);

			const result = context.withStatic(true);

			expect(result.isStatic).toBe(true);
			expect(result.isEffectivelyStatic).toBe(true);
			expect(context.isStatic).toBe(false);
		});

		it("round-trips the static flag through toJSON and fromJSON", () => {
			const context = buildContext("staging", [PROJECT_TAG]).withStatic(true);

			const restored = Context.fromJSON(context.toJSON());

			expect(restored.isStatic).toBe(true);
		});

		it("defaults to non-static when deserializing data saved without the flag", () => {
			const context = buildContext("staging", [PROJECT_TAG]);
			const { isStatic: _omitted, ...dataWithoutStatic } = context.toJSON();

			const restored = Context.fromJSON(dataWithoutStatic);

			expect(restored.isStatic).toBe(false);
		});
	});

	describe("Context.kiroConversationId", () => {
		it("is undefined for a freshly created context", () => {
			const context = buildContext("feature-x", [PROJECT_TAG]);

			expect(context.kiroConversationId).toBeUndefined();
		});

		it("returns a new context carrying the conversation id", () => {
			const context = buildContext("feature-x", [PROJECT_TAG]);

			const result = context.withKiroConversationId("conv-123");

			expect(result.kiroConversationId).toBe("conv-123");
			expect(context.kiroConversationId).toBeUndefined();
		});

		it("round-trips the conversation id through toJSON and fromJSON", () => {
			const context = buildContext("feature-x", [PROJECT_TAG]).withKiroConversationId("conv-123");

			const restored = Context.fromJSON(context.toJSON());

			expect(restored.kiroConversationId).toBe("conv-123");
		});
	});

	describe("Context.folderSegment", () => {
		it("falls back to the raw name when no folderName is stored (legacy contexts)", () => {
			const context = buildContext("Ajout d'un pipeline", [PROJECT_TAG]);

			expect(context.folderName).toBeUndefined();
			expect(context.folderSegment).toBe("Ajout d'un pipeline");
		});

		it("uses the stored slug when folderName is set", () => {
			const context = Context.create("Ajout d'un pipeline", [], "feature/x");

			expect(context.folderName).toBe("ajout-d-un-pipeline");
			expect(context.folderSegment).toBe("ajout-d-un-pipeline");
		});

		it("builds the worktree path from the folder segment, not the raw name", () => {
			const legacy = buildContext("My Feature", [PROJECT_TAG]);
			const slugged = Context.create("My Feature", [], "feature/x");
			const repo = new Repository("repo-1", "api", "./api");

			expect(legacy.getContextFolderPath("/p")).toBe("/p/.worktrees/My Feature");
			expect(slugged.getContextFolderPath("/p")).toBe("/p/.worktrees/my-feature");
			expect(slugged.getWorktreePath("/p", repo)).toBe("/p/.worktrees/my-feature/api");
		});

		it("round-trips folderName through toJSON and fromJSON", () => {
			const context = Context.create("My Feature", [], "feature/x");

			const restored = Context.fromJSON(context.toJSON());

			expect(restored.folderName).toBe("my-feature");
		});
	});

	describe("Context.workItemRef", () => {
		const WORK_ITEM_REF = { connectionId: "teamwork:acme.teamwork.com", workItemId: "101" };

		function buildContextFromTask(): Context {
			return new Context(
				"context-task",
				"task",
				[],
				false,
				undefined,
				{},
				undefined,
				undefined,
				false,
				WORK_ITEM_REF,
			);
		}

		it("recognizes the task it was created from", () => {
			expect(buildContextFromTask().isCreatedFrom(WORK_ITEM_REF)).toBe(true);
		});

		it("keeps the task reference through status changes and serialization", () => {
			const restored = Context.fromJSON(buildContextFromTask().withStatus("review").toJSON());

			expect(restored.workItemRef).toEqual(WORK_ITEM_REF);
		});

		it("ignores a malformed task reference when deserializing", () => {
			const data = { ...buildContextFromTask().toJSON(), workItemRef: { connectionId: 1 } };

			expect(Context.fromJSON(JSON.parse(JSON.stringify(data))).workItemRef).toBeUndefined();
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
	describe("branchNaming", () => {
		const legacyProjectData: ProjectData = {
			name: PROJECT_NAME,
			tag: PROJECT_TAG,
			path: PROJECT_PATH,
			repositories: [],
			contexts: [],
		};
		function buildProjectWithCustomBranchNaming(): Project {
			return Project.fromJSON({
				...legacyProjectData,
				branchNaming: { prefixes: ["ops"], allowNoPrefix: false },
			});
		}
		it("uses the default prefixes and allows no prefix for projects saved without the setting", () => {
			const project = Project.fromJSON(legacyProjectData);
			expect(project.branchNaming.prefixes).toEqual(DEFAULT_BRANCH_PREFIXES);
			expect(project.branchNaming.allowsNoPrefix).toBe(true);
		});
		it("preserves a custom setting and round-trips it through toJSON", () => {
			const project = buildProjectWithCustomBranchNaming();
			expect(project.branchNaming.prefixes).toEqual(["ops"]);
			expect(project.branchNaming.allowsNoPrefix).toBe(false);
			expect(project.toJSON().branchNaming).toEqual({ prefixes: ["ops"], allowNoPrefix: false });
		});
		it("preserves the setting when adding and removing contexts", () => {
			const project = buildProjectWithCustomBranchNaming();
			const context = buildContext("feature-x", [PROJECT_TAG]);
			const withContext = project.addContext(context);
			const withoutContext = withContext.removeContext(context.id);
			expect(withContext.branchNaming).toBe(project.branchNaming);
			expect(withoutContext.branchNaming).toBe(project.branchNaming);
		});
		it("uses the default policy when constructed without a setting", () => {
			expect(buildProject([]).branchNaming.toJSON()).toEqual(BranchNamingPolicy.default().toJSON());
		});
	});
});
