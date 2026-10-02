import type { WorkItemGroupId, WorkProjectId } from "./identifiers";
import type { WorkItemGroupKind } from "./WorkItemGroupKind";

export type WorkItemGroup = {
	readonly id: WorkItemGroupId;
	readonly projectId: WorkProjectId;
	readonly name: string;
	readonly description?: string;
	readonly kind: WorkItemGroupKind;
	readonly position?: number;
	readonly url?: string;
};
