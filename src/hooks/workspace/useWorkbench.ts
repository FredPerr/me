import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type PlacementView, type WorkbenchView } from "@/bindings";
import { WORKBENCH_QUERY_KEY } from "./workspaceQueryKeys";

const EMPTY_WORKBENCH: WorkbenchView = {
	entries: [],
	lastProjectTag: null,
};

/** The reconciled, ordered arrangement for the tab bar. */
export function useWorkbench() {
	const query = useQuery({
		queryKey: WORKBENCH_QUERY_KEY,
		queryFn: () => commands.workbenchGet(),
	});
	return {
		workbench: query.data ?? EMPTY_WORKBENCH,
		loading: query.isPending,
		error: query.error,
	};
}

/**
 * Each Workbench command returns the reconciled arrangement, so a mutation
 * writes it straight into the query; the `workbench-changed` event keeps other
 * windows in step.
 */
function useStoreWorkbench() {
	const queryClient = useQueryClient();
	return (workbench: WorkbenchView) => queryClient.setQueryData(WORKBENCH_QUERY_KEY, workbench);
}

type PlacementChange = { projectTag: string; placement: PlacementView };

export function useSetPlacement() {
	const storeWorkbench = useStoreWorkbench();
	return useMutation({
		mutationFn: ({ projectTag, placement }: PlacementChange) =>
			commands.workbenchSetPlacement(projectTag, placement),
		onSuccess: storeWorkbench,
	});
}

export function useSetLastProject() {
	const storeWorkbench = useStoreWorkbench();
	return useMutation({
		mutationFn: (projectTag: string | null) => commands.workbenchSetLastProject(projectTag),
		onSuccess: storeWorkbench,
	});
}

type ReorderChange = { placement: PlacementView; orderedProjectTags: string[] };

/** Reorders one placement group; drag-to-reorder (#96) wires it up. */
export function useReorder() {
	const storeWorkbench = useStoreWorkbench();
	return useMutation({
		mutationFn: ({ placement, orderedProjectTags }: ReorderChange) =>
			commands.workbenchReorder(placement, orderedProjectTags),
		onSuccess: storeWorkbench,
	});
}
