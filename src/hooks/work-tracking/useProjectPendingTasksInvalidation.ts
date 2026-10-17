import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { events } from "@/bindings";
import { pendingTasksQueryKey } from "./workTrackingQueryKeys";

/**
 * Refetches a project's pending task cards whenever Rust announces the project
 * changed, so creating a context from a task drops it from the board.
 */
export function useProjectPendingTasksInvalidation(): void {
	const queryClient = useQueryClient();

	useEffect(() => {
		let disposed = false;
		let stopListening: (() => void) | undefined;
		events.projectChanged
			.listen((event) => {
				const { tag } = event.payload;
				queryClient.invalidateQueries({ queryKey: pendingTasksQueryKey(tag) });
			})
			.then((unlisten) => {
				if (disposed) unlisten();
				else stopListening = unlisten;
			});
		return () => {
			disposed = true;
			stopListening?.();
		};
	}, [queryClient]);
}
