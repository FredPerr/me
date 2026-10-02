import type { ConnectionId } from "./identifiers";
import type { ProviderKind } from "./ProviderKind";

export type ProviderConnection = {
	readonly id: ConnectionId;
	readonly kind: ProviderKind;
	readonly baseUrl: string;
	readonly displayName: string;
	readonly credentialConfigured: boolean;
};
