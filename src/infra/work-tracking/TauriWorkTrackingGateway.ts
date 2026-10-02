import { type InvokeArgs, invoke } from "@tauri-apps/api/core";
import type {
	ConnectionId,
	WorkItemGroupId,
	WorkProjectId,
} from "@/domain/work-tracking/identifiers";
import type { Page } from "@/domain/work-tracking/Page";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import type { WorkItemGroup } from "@/domain/work-tracking/WorkItemGroup";
import type { WorkProject } from "@/domain/work-tracking/WorkProject";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import type {
	SaveConnectionInput,
	WorkTrackingGateway,
} from "@/domain/work-tracking/WorkTrackingGateway";

/**
 * Tauri adapter of the {@link WorkTrackingGateway} port. The backend makes every provider
 * request and keeps the API key in the keychain; the key only passes through `saveConnection`.
 */
export class TauriWorkTrackingGateway implements WorkTrackingGateway {
	listConnections(): Promise<ProviderConnection[]> {
		return this.call<ProviderConnection[]>("work_tracking_list_connections");
	}

	saveConnection({
		kind,
		baseUrl,
		displayName,
		apiKey,
	}: SaveConnectionInput): Promise<ProviderConnection> {
		return this.call<ProviderConnection>("work_tracking_save_connection", {
			input: { kind, baseUrl, displayName, apiKey },
		});
	}

	async removeConnection(connectionId: ConnectionId): Promise<void> {
		await this.call<null>("work_tracking_remove_connection", { connectionId });
	}

	listProjects(connectionId: ConnectionId, cursor?: string): Promise<Page<WorkProject>> {
		return this.call<Page<WorkProject>>("work_tracking_list_projects", {
			connectionId,
			cursor: cursor ?? null,
		});
	}

	listGroups(
		connectionId: ConnectionId,
		projectId: WorkProjectId,
		cursor?: string,
	): Promise<Page<WorkItemGroup>> {
		return this.call<Page<WorkItemGroup>>("work_tracking_list_groups", {
			connectionId,
			projectId,
			cursor: cursor ?? null,
		});
	}

	listItems(
		connectionId: ConnectionId,
		projectId: WorkProjectId,
		groupId: WorkItemGroupId,
		cursor?: string,
	): Promise<Page<WorkItem>> {
		return this.call<Page<WorkItem>>("work_tracking_list_items", {
			connectionId,
			projectId,
			groupId,
			cursor: cursor ?? null,
		});
	}

	private async call<T>(command: string, args?: InvokeArgs): Promise<T> {
		try {
			return await invoke<T>(command, args);
		} catch (rejection) {
			throw WorkTrackingError.fromUnknown(rejection);
		}
	}
}
