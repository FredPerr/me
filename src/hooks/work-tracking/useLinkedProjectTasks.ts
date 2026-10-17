import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
	type LinkedTask,
	listLinkedProjectTasks,
} from "@/application/work-tracking/listLinkedProjectTasks";
import type { RemoteProjectLinkView, WorkTrackingCommandError } from "@/bindings";
import { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";
import { isWorkTrackingCommandError } from "@/infra/commandError";
import { ConnectionsStatus, useWorkTrackingConnections } from "./useWorkTrackingConnections";

export type LinkedProjectTasks = {
	tasks: readonly LinkedTask[];
	loading: boolean;
	error: WorkTrackingCommandError | null;
	reload: () => void;
};

function asWorkTrackingError(error: unknown): WorkTrackingCommandError | null {
	return isWorkTrackingCommandError(error) ? error : null;
}

export function useLinkedProjectTasks(
	remoteProjectLinkViews: readonly RemoteProjectLinkView[],
	enabled: boolean,
): LinkedProjectTasks {
	const remoteProjectLinks = useMemo(
		() => RemoteProjectLinks.fromJSON(remoteProjectLinkViews),
		[remoteProjectLinkViews],
	);
	const { connections, status: connectionsStatus } = useWorkTrackingConnections();
	const [tasks, setTasks] = useState<readonly LinkedTask[]>([]);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<WorkTrackingCommandError | null>(null);
	const requestSequenceRef = useRef(0);

	const load = useCallback(async () => {
		requestSequenceRef.current += 1;
		const sequence = requestSequenceRef.current;
		setLoading(true);
		setError(null);
		try {
			const loaded = await listLinkedProjectTasks(remoteProjectLinks.toArray(), connections);
			if (sequence === requestSequenceRef.current) setTasks(loaded);
		} catch (loadError) {
			if (sequence === requestSequenceRef.current) setError(asWorkTrackingError(loadError));
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
