import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { events } from "@/bindings";
import { contextPullRequestsQueryKey } from "./useContextPullRequests";
import { projectPullRequestsQueryKey } from "./useProjectPullRequests";

/**
 * Refetches a project's pull requests, and those of its context branches,
 * whenever Rust announces the project changed.
 */
export function useProjectPullRequestsInvalidation(): void {
	const queryClient = useQueryClient();

	useEffect(() => {
		let disposed = false;
		let stopListening: (() => void) | undefined;
		events.projectChanged
			.listen((event) => {
				const { tag } = event.payload;
				queryClient.invalidateQueries({ queryKey: projectPullRequestsQueryKey(tag) });
				queryClient.invalidateQueries({ queryKey: contextPullRequestsQueryKey(tag) });
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
