import { useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type ProjectListView } from "@/bindings";
import { PROJECT_LIST_QUERY_KEY, projectQueryKey } from "./workspaceQueryKeys";

export function useProject(projectTag: string) {
	const queryClient = useQueryClient();
	return useQuery({
		queryKey: projectQueryKey(projectTag),
		queryFn: () => commands.workspaceGetProject(projectTag),
		initialData: () =>
			queryClient
				.getQueryData<ProjectListView>(PROJECT_LIST_QUERY_KEY)
				?.projects.find((project) => project.tag === projectTag),
		initialDataUpdatedAt: () => queryClient.getQueryState(PROJECT_LIST_QUERY_KEY)?.dataUpdatedAt,
	});
}
