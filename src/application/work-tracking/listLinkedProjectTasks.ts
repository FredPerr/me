import { type ConnectionView, commands, type Task, type TaskStatus } from "@/bindings";
import type { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";

export const MAX_PAGES_PER_PROJECT = 20;

const CLOSED_STATUSES: ReadonlySet<TaskStatus> = new Set<TaskStatus>(["done", "cancelled"]);

export type LinkedTask = {
	readonly connectionId: ConnectionView["id"];
	readonly task: Task;
};

async function listAllProjectTasks(link: RemoteProjectLink): Promise<Task[]> {
	const tasks: Task[] = [];
	let cursor: string | null = null;
	for (let pageIndex = 0; pageIndex < MAX_PAGES_PER_PROJECT; pageIndex += 1) {
		const page = await commands.workTrackingListProjectItems(
			link.connectionId,
			link.remoteProjectId,
			cursor,
		);
		tasks.push(...page.items);
		cursor = page.nextCursor;
		if (cursor === null) break;
	}
	return tasks;
}

/**
 * Loads the open tasks of every remote project linked to a local project. Links whose
 * connection is gone or lacks a credential are skipped, since they cannot be queried.
 */
export async function listLinkedProjectTasks(
	links: readonly RemoteProjectLink[],
	connections: readonly ConnectionView[],
): Promise<LinkedTask[]> {
	const queryableLinks = links.filter((link) =>
		connections.some(
			(connection) => connection.id === link.connectionId && connection.credentialConfigured,
		),
	);
	const tasksPerLink = await Promise.all(
		queryableLinks.map(async (link) => {
			const tasks = await listAllProjectTasks(link);
			return tasks.map((task) => ({ connectionId: link.connectionId, task }));
		}),
	);
	return tasksPerLink.flat().filter(({ task }) => !CLOSED_STATUSES.has(task.status));
}
