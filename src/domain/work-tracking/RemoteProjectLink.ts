import { InvalidInputReason } from "./InvalidInputReason";
import type { ConnectionId, WorkProjectId } from "./identifiers";
import { ProviderKind } from "./ProviderKind";
import { WorkTrackingError } from "./WorkTrackingError";
import { WorkTrackingErrorKind } from "./WorkTrackingErrorKind";

const MAX_ID_LENGTH = 300;
const MAX_NAME_LENGTH = 200;
const KNOWN_PROVIDER_KINDS: ReadonlySet<string> = new Set(Object.values(ProviderKind));

export type RemoteProjectLinkProps = {
	providerKind: ProviderKind;
	connectionId: string;
	remoteProjectId: string;
	remoteProjectName: string;
	remoteProjectUrl?: string;
};

export type RemoteProjectLinkData = {
	providerKind: ProviderKind;
	connectionId: string;
	remoteProjectId: string;
	remoteProjectName: string;
	remoteProjectUrl?: string;
	linkedAt: string;
};

function invalidLink(reason: InvalidInputReason): WorkTrackingError {
	return new WorkTrackingError(WorkTrackingErrorKind.InvalidInput, "Invalid remote project link", {
		reason,
	});
}

function requireText(value: unknown, maxLength: number): string {
	if (typeof value !== "string") throw invalidLink(InvalidInputReason.Required);
	const trimmed = value.trim();
	if (trimmed.length === 0) throw invalidLink(InvalidInputReason.Required);
	if (trimmed.length > maxLength) throw invalidLink(InvalidInputReason.TooLong);
	return trimmed;
}

function requireProviderKind(value: unknown): ProviderKind {
	if (typeof value !== "string" || !KNOWN_PROVIDER_KINDS.has(value)) {
		throw invalidLink(InvalidInputReason.Unsupported);
	}
	return value as ProviderKind;
}

function httpsUrlOrUndefined(value: unknown): string | undefined {
	if (typeof value !== "string") return undefined;
	const trimmed = value.trim();
	try {
		return new URL(trimmed).protocol === "https:" ? trimmed : undefined;
	} catch {
		return undefined;
	}
}

function requireTimestamp(value: unknown): string {
	if (typeof value !== "string" || Number.isNaN(Date.parse(value))) {
		throw invalidLink(InvalidInputReason.InvalidFormat);
	}
	return value;
}

export class RemoteProjectLink {
	private constructor(
		public readonly providerKind: ProviderKind,
		public readonly connectionId: ConnectionId,
		public readonly remoteProjectId: WorkProjectId,
		public readonly remoteProjectName: string,
		public readonly remoteProjectUrl: string | undefined,
		public readonly linkedAt: string,
	) {}

	static create(props: RemoteProjectLinkProps, now: Date): RemoteProjectLink {
		return RemoteProjectLink.build(props, now.toISOString());
	}

	static fromJSON(data: RemoteProjectLinkData): RemoteProjectLink {
		if (typeof data !== "object" || data === null) {
			throw invalidLink(InvalidInputReason.InvalidFormat);
		}
		return RemoteProjectLink.build(data, requireTimestamp(data.linkedAt));
	}

	private static build(props: RemoteProjectLinkProps, linkedAt: string): RemoteProjectLink {
		return new RemoteProjectLink(
			requireProviderKind(props.providerKind),
			requireText(props.connectionId, MAX_ID_LENGTH) as ConnectionId,
			requireText(props.remoteProjectId, MAX_ID_LENGTH) as WorkProjectId,
			requireText(props.remoteProjectName, MAX_NAME_LENGTH),
			httpsUrlOrUndefined(props.remoteProjectUrl),
			linkedAt,
		);
	}

	static identityKeyOf(
		providerKind: ProviderKind,
		connectionId: ConnectionId,
		remoteProjectId: WorkProjectId,
	): string {
		return `${providerKind}|${connectionId}|${remoteProjectId}`;
	}

	identityKey(): string {
		return RemoteProjectLink.identityKeyOf(
			this.providerKind,
			this.connectionId,
			this.remoteProjectId,
		);
	}

	sameIdentityAs(other: RemoteProjectLink): boolean {
		return this.identityKey() === other.identityKey();
	}

	withCachedDetails(remoteProjectName: string, remoteProjectUrl?: string): RemoteProjectLink {
		const refreshed = RemoteProjectLink.build(
			{
				providerKind: this.providerKind,
				connectionId: this.connectionId,
				remoteProjectId: this.remoteProjectId,
				remoteProjectName,
				remoteProjectUrl,
			},
			this.linkedAt,
		);
		const unchanged =
			refreshed.remoteProjectName === this.remoteProjectName &&
			refreshed.remoteProjectUrl === this.remoteProjectUrl;
		return unchanged ? this : refreshed;
	}

	toJSON(): RemoteProjectLinkData {
		return {
			providerKind: this.providerKind,
			connectionId: this.connectionId,
			remoteProjectId: this.remoteProjectId,
			remoteProjectName: this.remoteProjectName,
			...(this.remoteProjectUrl !== undefined && { remoteProjectUrl: this.remoteProjectUrl }),
			linkedAt: this.linkedAt,
		};
	}
}
