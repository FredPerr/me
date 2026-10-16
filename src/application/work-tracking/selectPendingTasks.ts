import type { ContextView } from "@/bindings";
import { isCreatedFromTask, type TaskRef } from "@/models/projectView";
import type { LinkedTask } from "./listLinkedProjectTasks";

export function taskRefOf(task: LinkedTask): TaskRef {
	return { connectionId: task.connectionId, taskId: task.item.id };
}

/**
 * Tasks still waiting for a context: those no existing context was created from, optionally
 * narrowed to titles matching the board search.
 */
export function selectPendingTasks(
	tasks: readonly LinkedTask[],
	contexts: readonly ContextView[],
	searchQuery = "",
): LinkedTask[] {
	const query = searchQuery.toLowerCase().trim();
	return tasks.filter((task) => {
		const taskRef = taskRefOf(task);
		if (contexts.some((context) => isCreatedFromTask(context, taskRef))) return false;
		return query === "" || task.item.title.toLowerCase().includes(query);
	});
}
