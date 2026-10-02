import { beforeEach, describe, expect, it, vi } from "vitest";
import { RemoteProjectLinkUpdateResult } from "@/application/work-tracking/RemoteProjectLinkRepository";
import { ProviderKind } from "@/domain/work-tracking/ProviderKind";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import { Context, Project, type ProjectData } from "@/models/Project";
import { ProjectDirectory } from "@/models/ProjectDirectory";
import { ProjectDirectoryRemoteProjectLinkRepository } from "./ProjectDirectoryRemoteProjectLinkRepository";

const storedProjects = new Map<string, ProjectData>();

vi.mock("@/models/ProjectDirectory", () => ({
	ProjectDirectory: {
		loadProject: vi.fn(async (projectTag: string) => {
			await Promise.resolve();
			const data = storedProjects.get(projectTag);
			return data ? Project.fromJSON(data) : null;
		}),
		saveProject: vi.fn(async (project: Project) => {
			await Promise.resolve();
			storedProjects.set(project.tag, project.toJSON());
		}),
	},
}));

const PROJECT_TAG = "my-project";

function storeProject(): void {
	const context = new Context("context-1", "default", [], true);
	const project = new Project("My Project", PROJECT_TAG, "/projects/my-project", [], [context]);
	storedProjects.set(PROJECT_TAG, project.toJSON());
}

function buildLink(remoteProjectId: string): RemoteProjectLink {
	return RemoteProjectLink.create(
		{
			providerKind: ProviderKind.Teamwork,
			connectionId: "teamwork:acme.teamwork.com",
			remoteProjectId,
			remoteProjectName: `Project ${remoteProjectId}`,
		},
		new Date("2024-01-15T10:00:00.000Z"),
	);
}

function storedRemoteProjectIds(): string[] {
	return (storedProjects.get(PROJECT_TAG)?.remoteProjectLinks ?? []).map(
		(link) => link.remoteProjectId,
	);
}

describe("ProjectDirectoryRemoteProjectLinkRepository", () => {
	beforeEach(() => {
		storedProjects.clear();
		vi.clearAllMocks();
	});

	it("returns NotFound without saving when the project does not exist", async () => {
		const repository = new ProjectDirectoryRemoteProjectLinkRepository();

		const result = await repository.update(PROJECT_TAG, (links) => links.add(buildLink("42")));

		expect(result).toBe(RemoteProjectLinkUpdateResult.NotFound);
		expect(ProjectDirectory.saveProject).not.toHaveBeenCalled();
	});

	it("returns Unchanged without saving when the mutation keeps the same instance", async () => {
		storeProject();
		const repository = new ProjectDirectoryRemoteProjectLinkRepository();

		const result = await repository.update(PROJECT_TAG, (links) => links);

		expect(result).toBe(RemoteProjectLinkUpdateResult.Unchanged);
		expect(ProjectDirectory.saveProject).not.toHaveBeenCalled();
	});

	it("saves the new links and keeps the other project fields", async () => {
		storeProject();
		const repository = new ProjectDirectoryRemoteProjectLinkRepository();

		const result = await repository.update(PROJECT_TAG, (links) => links.add(buildLink("42")));

		expect(result).toBe(RemoteProjectLinkUpdateResult.Saved);
		expect(storedRemoteProjectIds()).toEqual(["42"]);
		expect(storedProjects.get(PROJECT_TAG)?.contexts.map((context) => context.id)).toEqual([
			"context-1",
		]);
	});

	it("persists both links of two concurrent updates on the same project", async () => {
		storeProject();
		const repository = new ProjectDirectoryRemoteProjectLinkRepository();

		const results = await Promise.all([
			repository.update(PROJECT_TAG, (links) => links.add(buildLink("42"))),
			repository.update(PROJECT_TAG, (links) => links.add(buildLink("43"))),
		]);

		expect(results).toEqual([
			RemoteProjectLinkUpdateResult.Saved,
			RemoteProjectLinkUpdateResult.Saved,
		]);
		expect(storedRemoteProjectIds()).toEqual(["42", "43"]);
	});

	it("does not block the next update after a rejected one", async () => {
		storeProject();
		vi.mocked(ProjectDirectory.saveProject).mockRejectedValueOnce(new Error("disk full"));
		const repository = new ProjectDirectoryRemoteProjectLinkRepository();

		const failed = repository.update(PROJECT_TAG, (links) => links.add(buildLink("42")));
		const next = repository.update(PROJECT_TAG, (links) => links.add(buildLink("43")));

		await expect(failed).rejects.toThrow("disk full");
		await expect(next).resolves.toBe(RemoteProjectLinkUpdateResult.Saved);
		expect(storedRemoteProjectIds()).toEqual(["43"]);
	});
});
