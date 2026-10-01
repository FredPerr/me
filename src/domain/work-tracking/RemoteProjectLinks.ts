import type { ConnectionId, WorkProjectId } from "./identifiers";
import type { ProviderKind } from "./ProviderKind";
import { RemoteProjectLink, type RemoteProjectLinkData } from "./RemoteProjectLink";

export class RemoteProjectLinks {
	private static readonly EMPTY = new RemoteProjectLinks([]);

	private constructor(private readonly links: readonly RemoteProjectLink[]) {}

	static empty(): RemoteProjectLinks {
		return RemoteProjectLinks.EMPTY;
	}

	static fromJSON(data: unknown): RemoteProjectLinks {
		if (!Array.isArray(data)) return RemoteProjectLinks.empty();
		const links: RemoteProjectLink[] = [];
		const seenIdentityKeys = new Set<string>();
		for (const entry of data) {
			let link: RemoteProjectLink;
			try {
				link = RemoteProjectLink.fromJSON(entry as RemoteProjectLinkData);
			} catch {
				console.warn("Skipping invalid remote project link");
				continue;
			}
			const identityKey = link.identityKey();
			if (seenIdentityKeys.has(identityKey)) continue;
			seenIdentityKeys.add(identityKey);
			links.push(link);
		}
		return links.length === 0 ? RemoteProjectLinks.empty() : new RemoteProjectLinks(links);
	}

	add(link: RemoteProjectLink): RemoteProjectLinks {
		const existingIndex = this.links.findIndex((existing) => existing.sameIdentityAs(link));
		if (existingIndex === -1) return new RemoteProjectLinks([...this.links, link]);
		const existing = this.links[existingIndex];
		const refreshed = existing.withCachedDetails(link.remoteProjectName, link.remoteProjectUrl);
		if (refreshed === existing) return this;
		return new RemoteProjectLinks(
			this.links.map((current, index) => (index === existingIndex ? refreshed : current)),
		);
	}

	remove(identityKey: string): RemoteProjectLinks {
		const remaining = this.links.filter((link) => link.identityKey() !== identityKey);
		return remaining.length === this.links.length ? this : new RemoteProjectLinks(remaining);
	}

	has(
		providerKind: ProviderKind,
		connectionId: ConnectionId,
		remoteProjectId: WorkProjectId,
	): boolean {
		return this.links.some(
			(link) =>
				link.providerKind === providerKind &&
				link.connectionId === connectionId &&
				link.remoteProjectId === remoteProjectId,
		);
	}

	toArray(): readonly RemoteProjectLink[] {
		return this.links;
	}

	toJSON(): RemoteProjectLinkData[] {
		return this.links.map((link) => link.toJSON());
	}
}
