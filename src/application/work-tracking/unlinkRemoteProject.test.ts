import { describe, expect, it, vi } from "vitest";
import { ProviderKind } from "@/domain/work-tracking/ProviderKind";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";
import {
	type RemoteProjectLinkRepository,
	RemoteProjectLinkUpdateResult,
} from "./RemoteProjectLinkRepository";
import { unlinkRemoteProject } from "./unlinkRemoteProject";

const PROJECT_TAG = "my-project";

class InMemoryRemoteProjectLinkRepository implements RemoteProjectLinkRepository {
	readonly writes: RemoteProjectLinks[] = [];
	readonly update = vi.fn(
		async (
			projectTag: string,
			mutate: (links: RemoteProjectLinks) => RemoteProjectLinks,
		): Promise<RemoteProjectLinkUpdateResult> => {
			const current = this.linksByTag.get(projectTag);
			if (current === undefined) return RemoteProjectLinkUpdateResult.NotFound;
			const next = mutate(current);
			if (next === current) return RemoteProjectLinkUpdateResult.Unchanged;
			this.linksByTag.set(projectTag, next);
			this.writes.push(next);
			return RemoteProjectLinkUpdateResult.Saved;
		},
	);

	constructor(private readonly linksByTag: Map<string, RemoteProjectLinks>) {}

	linksOf(projectTag: string): RemoteProjectLinks | undefined {
		return this.linksByTag.get(projectTag);
	}
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

describe("unlinkRemoteProject", () => {
	it("removes only the given link (AC16)", async () => {
		const first = buildLink("42");
		const repository = new InMemoryRemoteProjectLinkRepository(
			new Map([[PROJECT_TAG, RemoteProjectLinks.empty().add(first).add(buildLink("43"))]]),
		);

		await unlinkRemoteProject(repository, {
			projectTag: PROJECT_TAG,
			identityKey: first.identityKey(),
		});

		expect(repository.writes).toHaveLength(1);
		expect(
			repository
				.linksOf(PROJECT_TAG)
				?.toArray()
				.map((link) => link.remoteProjectId),
		).toEqual(["43"]);
	});

	it("does not write when the link is absent", async () => {
		const repository = new InMemoryRemoteProjectLinkRepository(
			new Map([[PROJECT_TAG, RemoteProjectLinks.empty().add(buildLink("43"))]]),
		);

		await unlinkRemoteProject(repository, {
			projectTag: PROJECT_TAG,
			identityKey: buildLink("42").identityKey(),
		});

		expect(repository.update).toHaveBeenCalledTimes(1);
		expect(repository.writes).toHaveLength(0);
	});

	it("throws when the project is not found", async () => {
		const repository = new InMemoryRemoteProjectLinkRepository(new Map());

		await expect(
			unlinkRemoteProject(repository, { projectTag: PROJECT_TAG, identityKey: "x" }),
		).rejects.toThrow("Project not found");
	});
});
