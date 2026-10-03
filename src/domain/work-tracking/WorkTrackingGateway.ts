import type { ConnectionId, WorkItemGroupId, WorkProjectId } from "./identifiers";
import type { Page } from "./Page";
import type { ProviderConnection } from "./ProviderConnection";
import type { ProviderKind } from "./ProviderKind";
import type { WorkItem } from "./WorkItem";
import type { WorkItemGroup } from "./WorkItemGroup";
import type { WorkProject } from "./WorkProject";

export type SaveConnectionInput = {
	readonly kind: ProviderKind;
	readonly baseUrl: string;
	readonly displayName?: string;
	readonly apiKey: string;
};

export interface WorkTrackingGateway {
	listConnections(): Promise<ProviderConnection[]>;
	saveConnection(input: SaveConnectionInput): Promise<ProviderConnection>;
	removeConnection(connectionId: ConnectionId): Promise<void>;
	listProjects(connectionId: ConnectionId, cursor?: string): Promise<Page<WorkProject>>;
	listGroups(
		connectionId: ConnectionId,
		projectId: WorkProjectId,
		cursor?: string,
	): Promise<Page<WorkItemGroup>>;
	listItems(
		connectionId: ConnectionId,
		projectId: WorkProjectId,
		groupId: WorkItemGroupId,
		cursor?: string,
	): Promise<Page<WorkItem>>;
	listProjectItems(
		connectionId: ConnectionId,
		projectId: WorkProjectId,
		cursor?: string,
	): Promise<Page<WorkItem>>;
}
