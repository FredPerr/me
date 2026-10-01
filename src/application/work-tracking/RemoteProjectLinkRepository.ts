import type { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";

export enum RemoteProjectLinkUpdateResult {
	Saved = "saved",
	Unchanged = "unchanged",
	NotFound = "notFound",
}

export interface RemoteProjectLinkRepository {
	/** Applies `mutate` to the freshly loaded links; writes only if the result is a new instance. */
	update(
		projectTag: string,
		mutate: (links: RemoteProjectLinks) => RemoteProjectLinks,
	): Promise<RemoteProjectLinkUpdateResult>;
}
