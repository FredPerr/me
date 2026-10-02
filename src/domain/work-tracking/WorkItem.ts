import type { WorkItemGroupId, WorkItemId, WorkProjectId } from "./identifiers";
import type { Label } from "./Label";
import type { Person } from "./Person";
import type { WorkItemPriority } from "./WorkItemPriority";
import type { WorkItemStatus } from "./WorkItemStatus";

export type WorkItem = {
	readonly id: WorkItemId;
	readonly projectId: WorkProjectId;
	readonly groupId?: WorkItemGroupId;
	readonly parentId?: WorkItemId;
	readonly title: string;
	readonly description?: string;
	readonly status: WorkItemStatus;
	readonly rawStatus: string;
	readonly priority: WorkItemPriority;
	readonly rawPriority?: string;
	readonly assignees: readonly Person[];
	readonly labels: readonly Label[];
	readonly dueDate?: string;
	readonly url: string;
	readonly updatedAt?: string;
};
