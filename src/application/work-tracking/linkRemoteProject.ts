import {
	RemoteProjectLink,
	type RemoteProjectLinkProps,
} from "@/domain/work-tracking/RemoteProjectLink";
import {
	type RemoteProjectLinkRepository,
	RemoteProjectLinkUpdateResult,
} from "./RemoteProjectLinkRepository";

type LinkRemoteProjectCommand = RemoteProjectLinkProps & {
	projectTag: string;
};

export async function linkRemoteProject(
	repository: RemoteProjectLinkRepository,
	{ projectTag, ...linkProps }: LinkRemoteProjectCommand,
	now: Date = new Date(),
): Promise<void> {
	const link = RemoteProjectLink.create(linkProps, now);
	const result = await repository.update(projectTag, (links) => links.add(link));
	if (result === RemoteProjectLinkUpdateResult.NotFound) {
		throw new Error("Project not found");
	}
}
