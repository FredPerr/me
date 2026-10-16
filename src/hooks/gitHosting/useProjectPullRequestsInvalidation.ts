import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { events } from "@/bindings";
import { projectPullRequestsQueryKey } from "./useProjectPullRequests";

/** Refetches a project's pull requests whenever Rust announces the project changed. */
export function useProjectPullRequestsInvalidation(): void {
	const queryClient = useQueryClient();

	useEffect(() => {
		let disposed = false;
		let stopListening: (() => void) | undefined;
		events.projectChanged
			.listen((event) =>
				queryClient.invalidateQueries({
					queryKey: projectPullRequestsQueryKey(event.payload.tag),
				}),
			)
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
