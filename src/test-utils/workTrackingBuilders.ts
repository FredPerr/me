import type { Task } from "@/bindings";
import type { LinkedTask } from "@/models/work-tracking/pendingTask";

export const DEFAULT_CONNECTION_ID = "teamwork:acme.teamwork.com";
const DEFAULT_REMOTE_PROJECT_ID = "42";

export function buildTask(overrides?: Partial<Task>): Task {
	return {
		id: "1",
		projectId: DEFAULT_REMOTE_PROJECT_ID,
		groupId: null,
		parentId: null,
		title: "Add login",
		description: null,
		status: "todo",
		rawStatus: "new",
		priority: "none",
		rawPriority: null,
		assignees: [],
		labels: [],
		dueDate: null,
		url: "https://acme.teamwork.com/app/tasks/1",
		updatedAt: null,
		...overrides,
	};
}

export function buildLinkedTask(task: Partial<Task> = {}): LinkedTask {
	return { connectionId: DEFAULT_CONNECTION_ID, task: buildTask(task) };
}
