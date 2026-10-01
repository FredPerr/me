import {
	type RemoteProjectLinkRepository,
	RemoteProjectLinkUpdateResult,
} from "@/application/work-tracking/RemoteProjectLinkRepository";
import type { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";
import { ProjectDirectory } from "@/models/ProjectDirectory";

type LinksMutation = (links: RemoteProjectLinks) => RemoteProjectLinks;

export class ProjectDirectoryRemoteProjectLinkRepository implements RemoteProjectLinkRepository {
	private readonly queues = new Map<string, Promise<unknown>>();

	update(projectTag: string, mutate: LinksMutation): Promise<RemoteProjectLinkUpdateResult> {
		const previous = this.queues.get(projectTag) ?? Promise.resolve();
		const current = previous
			.catch(() => undefined)
			.then(() => this.loadMutateAndSave(projectTag, mutate));
		this.queues.set(projectTag, current);
		const releaseQueue = () => {
			if (this.queues.get(projectTag) === current) this.queues.delete(projectTag);
		};
		current.then(releaseQueue, releaseQueue);
		return current;
	}

	private async loadMutateAndSave(
		projectTag: string,
		mutate: LinksMutation,
	): Promise<RemoteProjectLinkUpdateResult> {
		const project = await ProjectDirectory.loadProject(projectTag);
		if (project === null) return RemoteProjectLinkUpdateResult.NotFound;
		const next = mutate(project.remoteProjectLinks);
		if (next === project.remoteProjectLinks) return RemoteProjectLinkUpdateResult.Unchanged;
		await ProjectDirectory.saveProject(project.withRemoteProjectLinks(next));
		return RemoteProjectLinkUpdateResult.Saved;
	}
}
