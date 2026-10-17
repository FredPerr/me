import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useRef } from "react";
import {
	type ConnectionView,
	commands,
	type SaveConnectionInput,
	type WorkTrackingCommandError,
} from "@/bindings";
import { isWorkTrackingCommandError } from "@/infra/commandError";
import { SHOWS_ITS_OWN_ERROR } from "@/infra/queryClient";
import { CONNECTIONS_QUERY_KEY } from "./workTrackingQueryKeys";

export enum ConnectionsStatus {
	Loading = "loading",
	Ready = "ready",
	Error = "error",
}

export type WorkTrackingConnections = {
	connections: readonly ConnectionView[];
	status: ConnectionsStatus;
	error: WorkTrackingCommandError | null;
	revision: number;
	save: (input: SaveConnectionInput) => Promise<ConnectionView>;
	remove: (connectionId: string) => Promise<void>;
	reload: () => Promise<void>;
};

const NO_CONNECTIONS: ConnectionView[] = [];

function toStatus(isPending: boolean, isError: boolean): ConnectionsStatus {
	if (isError) return ConnectionsStatus.Error;
	if (isPending) return ConnectionsStatus.Loading;
	return ConnectionsStatus.Ready;
}

function asWorkTrackingError(error: unknown): WorkTrackingCommandError | null {
	return isWorkTrackingCommandError(error) ? error : null;
}

/**
 * The configured work tracker connections, with helpers to add or remove one.
 * `revision` changes after every successful mutation, so views keyed on it
 * (the paged lists) reload against the new connection set.
 */
export function useWorkTrackingConnections(): WorkTrackingConnections {
	const queryClient = useQueryClient();
	const revisionRef = useRef(0);

	const query = useQuery({
		queryKey: CONNECTIONS_QUERY_KEY,
		queryFn: () => commands.workTrackingListConnections(),
	});

	const invalidate = useCallback(() => {
		revisionRef.current += 1;
		return queryClient.invalidateQueries({ queryKey: CONNECTIONS_QUERY_KEY });
	}, [queryClient]);

	// The connection form and the modals show their own failures.
	const saveMutation = useMutation({
		mutationFn: (input: SaveConnectionInput) => commands.workTrackingSaveConnection(input),
		onSuccess: invalidate,
		meta: { [SHOWS_ITS_OWN_ERROR]: true },
	});
	const removeMutation = useMutation({
		mutationFn: (connectionId: string) => commands.workTrackingRemoveConnection(connectionId),
		onSuccess: invalidate,
		meta: { [SHOWS_ITS_OWN_ERROR]: true },
	});

	const save = useCallback(
		(input: SaveConnectionInput) => saveMutation.mutateAsync(input),
		[saveMutation],
	);
	const remove = useCallback(
		async (connectionId: string) => {
			await removeMutation.mutateAsync(connectionId);
		},
		[removeMutation],
	);
	const reload = useCallback(async () => {
		await query.refetch();
	}, [query]);

	return {
		connections: query.data ?? NO_CONNECTIONS,
		status: toStatus(query.isPending, query.isError),
		error: asWorkTrackingError(query.error),
		revision: revisionRef.current,
		save,
		remove,
		reload,
	};
}
