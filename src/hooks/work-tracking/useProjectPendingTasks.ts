import { queryOptions, useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { type ConnectionId, commands, type WorkTrackingCommandError } from "@/bindings";
import { isWorkTrackingCommandError } from "@/infra/commandError";
import { type LinkedTask, linkedTaskOf } from "@/models/work-tracking/pendingTask";
import { pendingTasksQueryKey } from "./workTrackingQueryKeys";

export type ProjectPendingTasks = {
	tasks: readonly LinkedTask[];
	unavailableConnections: readonly ConnectionId[];
	loading: boolean;
	error: WorkTrackingCommandError | null;
};

const NO_TASKS: readonly LinkedTask[] = [];
const NO_CONNECTIONS: readonly ConnectionId[] = [];

function projectPendingTasksQuery(projectTag: string, enabled: boolean) {
	return queryOptions({
		queryKey: pendingTasksQueryKey(projectTag),
		queryFn: () => commands.workTrackingPendingTasks(projectTag),
		enabled,
	});
}

function asWorkTrackingError(error: unknown): WorkTrackingCommandError | null {
	return isWorkTrackingCommandError(error) ? error : null;
}

/**
 * The board's pending task cards for a project: the open tasks of its linked
 * remote projects that no context came from yet, computed in one Rust call.
 * `unavailableConnections` lists connections that were skipped so the user can
 * reconnect.
 */
export function useProjectPendingTasks(projectTag: string, enabled: boolean): ProjectPendingTasks {
	const query = useQuery(projectPendingTasksQuery(projectTag, enabled));

	const tasks = useMemo(() => query.data?.tasks.map(linkedTaskOf) ?? NO_TASKS, [query.data?.tasks]);

	return {
		tasks: enabled ? tasks : NO_TASKS,
		unavailableConnections: query.data?.unavailableConnections ?? NO_CONNECTIONS,
		loading: enabled && query.isPending,
		error: asWorkTrackingError(query.error),
	};
}
