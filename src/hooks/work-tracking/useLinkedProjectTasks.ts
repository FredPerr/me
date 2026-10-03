import { useCallback, useEffect, useRef, useState } from "react";
import {
	type LinkedTask,
	listLinkedProjectTasks,
} from "@/application/work-tracking/listLinkedProjectTasks";
import type { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { workTrackingGateway } from "@/infra/work-tracking/workTracking";
import { ConnectionsStatus, useWorkTrackingConnections } from "./useWorkTrackingConnections";

export type LinkedProjectTasks = {
	tasks: readonly LinkedTask[];
	loading: boolean;
	error: WorkTrackingError | null;
	reload: () => void;
};

export function useLinkedProjectTasks(
	remoteProjectLinks: RemoteProjectLinks,
	enabled: boolean,
): LinkedProjectTasks {
	const { connections, status: connectionsStatus } = useWorkTrackingConnections();
	const [tasks, setTasks] = useState<readonly LinkedTask[]>([]);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<WorkTrackingError | null>(null);
	const requestSequenceRef = useRef(0);

	const load = useCallback(async () => {
		requestSequenceRef.current += 1;
		const sequence = requestSequenceRef.current;
		setLoading(true);
		setError(null);
		try {
			const loaded = await listLinkedProjectTasks(
				workTrackingGateway,
				remoteProjectLinks.toArray(),
				connections,
			);
			if (sequence === requestSequenceRef.current) setTasks(loaded);
		} catch (loadError) {
			if (sequence === requestSequenceRef.current) {
				setError(WorkTrackingError.fromUnknown(loadError));
			}
		} finally {
			if (sequence === requestSequenceRef.current) setLoading(false);
		}
	}, [remoteProjectLinks, connections]);

	useEffect(() => {
		if (!enabled || connectionsStatus !== ConnectionsStatus.Ready) return;
		void load();
		return () => {
			requestSequenceRef.current += 1;
		};
	}, [enabled, connectionsStatus, load]);

	const reload = useCallback(() => void load(), [load]);

	return { tasks: enabled ? tasks : [], loading: enabled && loading, error, reload };
}
