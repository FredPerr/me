import type { Context, WorkItemRef } from "@/models/Project";
import type { LinkedTask } from "./listLinkedProjectTasks";

export function workItemRefOf(task: LinkedTask): WorkItemRef {
	return { connectionId: task.connectionId, workItemId: task.item.id };
}

/**
 * Tasks still waiting for a context: those no existing context was created from, optionally
 * narrowed to titles matching the board search.
 */
export function selectPendingTasks(
	tasks: readonly LinkedTask[],
	contexts: readonly Context[],
	searchQuery = "",
): LinkedTask[] {
	const query = searchQuery.toLowerCase().trim();
	return tasks.filter((task) => {
		const ref = workItemRefOf(task);
		if (contexts.some((context) => context.isCreatedFrom(ref))) return false;
		return query === "" || task.item.title.toLowerCase().includes(query);
	});
}
