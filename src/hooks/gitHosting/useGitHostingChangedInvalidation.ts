import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { events } from "@/bindings";
import { GIT_HOSTING_QUERY_KEY } from "./gitHostingQueryKeys";

/** Refetches every Git hosting query whenever Rust announces a sign-in or sign-out. */
export function useGitHostingChangedInvalidation(): void {
	const queryClient = useQueryClient();

	useEffect(() => {
		let disposed = false;
		let stopListening: (() => void) | undefined;
		events.gitHostingChanged
			.listen(() => queryClient.invalidateQueries({ queryKey: GIT_HOSTING_QUERY_KEY }))
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
