import type { LinkedTask } from "@/application/work-tracking/listLinkedProjectTasks";
import type { ConnectionView, Task } from "@/bindings";

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

export function buildConnection(overrides?: Partial<ConnectionView>): ConnectionView {
	return {
		id: DEFAULT_CONNECTION_ID,
		kind: "teamwork",
		baseUrl: "https://acme.teamwork.com",
		displayName: "acme.teamwork.com",
		credentialConfigured: true,
		...overrides,
	};
}

export function buildLinkedTask(task: Partial<Task> = {}): LinkedTask {
	return { connectionId: DEFAULT_CONNECTION_ID, task: buildTask(task) };
}
