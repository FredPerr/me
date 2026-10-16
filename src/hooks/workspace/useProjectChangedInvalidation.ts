import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { events } from "@/bindings";
import { invalidateProject } from "./projectCache";

/** Refetches a project and the project list whenever Rust announces the project changed. */
export function useProjectChangedInvalidation(): void {
	const queryClient = useQueryClient();

	useEffect(() => {
		let disposed = false;
		let stopListening: (() => void) | undefined;
		events.projectChanged
			.listen((event) => invalidateProject(queryClient, event.payload.tag))
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
