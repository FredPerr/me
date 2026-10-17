import type { ConnectionView, PendingTaskView, Task } from "@/bindings";
import type { TaskRef } from "@/models/projectView";

/** A task from a linked remote project, tagged with the connection it lives on. */
export type LinkedTask = {
	readonly connectionId: ConnectionView["id"];
	readonly task: Task;
};

export function taskRefOf(task: LinkedTask): TaskRef {
	return { connectionId: task.connectionId, taskId: task.task.id };
}

export function linkedTaskOf(pending: PendingTaskView): LinkedTask {
	return { connectionId: pending.connectionId, task: pending.task };
}
