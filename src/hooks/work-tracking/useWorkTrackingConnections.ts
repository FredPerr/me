import { useCallback, useEffect, useRef, useState } from "react";
import type { ConnectionId } from "@/domain/work-tracking/identifiers";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import type { SaveConnectionInput } from "@/domain/work-tracking/WorkTrackingGateway";
import { workTrackingGateway } from "@/infra/work-tracking/workTracking";

export enum ConnectionsStatus {
	Loading = "loading",
	Ready = "ready",
	Error = "error",
}

export type WorkTrackingConnections = {
	connections: readonly ProviderConnection[];
	status: ConnectionsStatus;
	error: WorkTrackingError | null;
	revision: number;
	save: (input: SaveConnectionInput) => Promise<ProviderConnection>;
	remove: (connectionId: ConnectionId) => Promise<void>;
	reload: () => Promise<void>;
};

export function useWorkTrackingConnections(): WorkTrackingConnections {
	const [connections, setConnections] = useState<readonly ProviderConnection[]>([]);
	const [status, setStatus] = useState(ConnectionsStatus.Loading);
	const [error, setError] = useState<WorkTrackingError | null>(null);
	const [revision, setRevision] = useState(0);
	const requestSequenceRef = useRef(0);

	const fetchConnections = useCallback(async () => {
		requestSequenceRef.current += 1;
		const sequence = requestSequenceRef.current;
		try {
			const loaded = await workTrackingGateway.listConnections();
			if (sequence !== requestSequenceRef.current) return;
			setConnections(loaded);
			setError(null);
			setStatus(ConnectionsStatus.Ready);
		} catch (loadError) {
			if (sequence !== requestSequenceRef.current) return;
			setError(WorkTrackingError.fromUnknown(loadError));
			setStatus(ConnectionsStatus.Error);
		}
	}, []);

	const reload = useCallback(async () => {
		setStatus(ConnectionsStatus.Loading);
		await fetchConnections();
	}, [fetchConnections]);

	const save = useCallback(
		async (input: SaveConnectionInput) => {
			const saved = await workTrackingGateway.saveConnection(input);
			await fetchConnections();
			setRevision((current) => current + 1);
			return saved;
		},
		[fetchConnections],
	);

	const remove = useCallback(
		async (connectionId: ConnectionId) => {
			await workTrackingGateway.removeConnection(connectionId);
			await fetchConnections();
			setRevision((current) => current + 1);
		},
		[fetchConnections],
	);

	useEffect(() => {
		void fetchConnections();
	}, [fetchConnections]);

	return { connections, status, error, revision, save, remove, reload };
}
