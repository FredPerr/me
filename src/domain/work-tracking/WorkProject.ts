import type { ConnectionId, WorkProjectId } from "./identifiers";
import type { WorkProjectStatus } from "./WorkProjectStatus";

export type WorkProject = {
	readonly id: WorkProjectId;
	readonly connectionId: ConnectionId;
	readonly name: string;
	readonly description?: string;
	readonly status: WorkProjectStatus;
	readonly url: string;
};
