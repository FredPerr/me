import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { events } from "@/bindings";
import { WORKBENCH_QUERY_KEY } from "./workspaceQueryKeys";

/** Refetches the Workbench whenever Rust announces the arrangement changed. */
export function useWorkbenchChangedInvalidation(): void {
	const queryClient = useQueryClient();

	useEffect(() => {
		let disposed = false;
		let stopListening: (() => void) | undefined;
		events.workbenchChanged
			.listen(() => queryClient.invalidateQueries({ queryKey: WORKBENCH_QUERY_KEY }))
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
