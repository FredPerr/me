import {
	type RemoteProjectLinkRepository,
	RemoteProjectLinkUpdateResult,
} from "./RemoteProjectLinkRepository";

type UnlinkRemoteProjectCommand = {
	projectTag: string;
	identityKey: string;
};

export async function unlinkRemoteProject(
	repository: RemoteProjectLinkRepository,
	{ projectTag, identityKey }: UnlinkRemoteProjectCommand,
): Promise<void> {
	const result = await repository.update(projectTag, (links) => links.remove(identityKey));
	if (result === RemoteProjectLinkUpdateResult.NotFound) {
		throw new Error("Project not found");
	}
}
