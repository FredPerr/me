import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { ProjectView } from "@/bindings";
import { storeProject } from "./projectCache";

/** A workspace command that returns the changed Project, which then replaces the cached one. */
export function useProjectMutation<Variables>(
	changeProject: (variables: Variables) => Promise<ProjectView>,
) {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: changeProject,
		onSuccess: (project) => storeProject(queryClient, project),
	});
}
