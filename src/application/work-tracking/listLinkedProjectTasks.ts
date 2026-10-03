import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import type { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import { WorkItemStatus } from "@/domain/work-tracking/WorkItemStatus";
import type { WorkTrackingGateway } from "@/domain/work-tracking/WorkTrackingGateway";

export const MAX_PAGES_PER_PROJECT = 20;

const CLOSED_STATUSES: ReadonlySet<WorkItemStatus> = new Set([
	WorkItemStatus.Done,
	WorkItemStatus.Cancelled,
]);

export type LinkedTask = {
	readonly connectionId: ProviderConnection["id"];
	readonly item: WorkItem;
};

type ProjectTasksGateway = Pick<WorkTrackingGateway, "listProjectItems">;

async function listAllProjectItems(
	gateway: ProjectTasksGateway,
	link: RemoteProjectLink,
): Promise<WorkItem[]> {
	const items: WorkItem[] = [];
	let cursor: string | undefined;
	for (let pageIndex = 0; pageIndex < MAX_PAGES_PER_PROJECT; pageIndex += 1) {
		const page = await gateway.listProjectItems(link.connectionId, link.remoteProjectId, cursor);
		items.push(...page.items);
		cursor = page.nextCursor;
		if (cursor === undefined) break;
	}
	return items;
}

/**
 * Loads the open tasks of every remote project linked to a local project. Links whose
 * connection is gone or lacks a credential are skipped, since they cannot be queried.
 */
export async function listLinkedProjectTasks(
	gateway: ProjectTasksGateway,
	links: readonly RemoteProjectLink[],
	connections: readonly ProviderConnection[],
): Promise<LinkedTask[]> {
	const queryableLinks = links.filter((link) =>
		connections.some(
			(connection) => connection.id === link.connectionId && connection.credentialConfigured,
		),
	);
	const itemsPerLink = await Promise.all(
		queryableLinks.map(async (link) => {
			const items = await listAllProjectItems(gateway, link);
			return items.map((item) => ({ connectionId: link.connectionId, item }));
		}),
	);
	return itemsPerLink.flat().filter(({ item }) => !CLOSED_STATUSES.has(item.status));
}
