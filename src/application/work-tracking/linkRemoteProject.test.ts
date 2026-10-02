import { describe, expect, it, vi } from "vitest";
import { InvalidInputReason } from "@/domain/work-tracking/InvalidInputReason";
import type { ConnectionId, WorkProjectId } from "@/domain/work-tracking/identifiers";
import { ProviderKind } from "@/domain/work-tracking/ProviderKind";
import { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { WorkTrackingErrorKind } from "@/domain/work-tracking/WorkTrackingErrorKind";
import { linkRemoteProject } from "./linkRemoteProject";
import {
	type RemoteProjectLinkRepository,
	RemoteProjectLinkUpdateResult,
} from "./RemoteProjectLinkRepository";

const PROJECT_TAG = "my-project";
const CONNECTION_ID = "teamwork:acme.teamwork.com";
const NOW = new Date("2024-01-15T10:00:00.000Z");

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

function buildCommand(remoteProjectId = "42") {
	return {
		projectTag: PROJECT_TAG,
		providerKind: ProviderKind.Teamwork,
		connectionId: CONNECTION_ID,
		remoteProjectId,
		remoteProjectName: "Website",
		remoteProjectUrl: `https://acme.teamwork.com/app/projects/${remoteProjectId}`,
	};
}

describe("linkRemoteProject", () => {
	it("saves the new link on the project", async () => {
		const repository = new InMemoryRemoteProjectLinkRepository(
			new Map([[PROJECT_TAG, RemoteProjectLinks.empty()]]),
		);

		await linkRemoteProject(repository, buildCommand(), NOW);

		const links = repository.linksOf(PROJECT_TAG);
		expect(repository.writes).toHaveLength(1);
		expect(
			links?.has(ProviderKind.Teamwork, CONNECTION_ID as ConnectionId, "42" as WorkProjectId),
		).toBe(true);
		expect(links?.toArray()[0].linkedAt).toBe(NOW.toISOString());
	});

	it("does not write when the link already exists unchanged", async () => {
		const repository = new InMemoryRemoteProjectLinkRepository(
			new Map([[PROJECT_TAG, RemoteProjectLinks.empty()]]),
		);
		await linkRemoteProject(repository, buildCommand(), NOW);

		await linkRemoteProject(repository, buildCommand(), new Date());

		expect(repository.update).toHaveBeenCalledTimes(2);
		expect(repository.writes).toHaveLength(1);
	});

	it("throws when the project is not found", async () => {
		const repository = new InMemoryRemoteProjectLinkRepository(new Map());

		await expect(linkRemoteProject(repository, buildCommand(), NOW)).rejects.toThrow(
			"Project not found",
		);
	});

	it("throws invalidInput before any update for invalid link props", async () => {
		const repository = new InMemoryRemoteProjectLinkRepository(
			new Map([[PROJECT_TAG, RemoteProjectLinks.empty()]]),
		);

		const result = linkRemoteProject(repository, buildCommand(""), NOW);

		await expect(result).rejects.toBeInstanceOf(WorkTrackingError);
		await expect(result).rejects.toMatchObject({
			kind: WorkTrackingErrorKind.InvalidInput,
			reason: InvalidInputReason.Required,
		});
		expect(repository.update).not.toHaveBeenCalled();
	});
});
