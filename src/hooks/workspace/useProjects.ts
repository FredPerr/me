import { useQuery } from "@tanstack/react-query";
import { commands, type ProjectListView } from "@/bindings";
import { PROJECT_LIST_QUERY_KEY } from "./workspaceQueryKeys";

const EMPTY_LISTING: ProjectListView = {
	projects: [],
	unreadableProjects: [],
};

export function useProjects() {
	const query = useQuery({
		queryKey: PROJECT_LIST_QUERY_KEY,
		queryFn: () => commands.workspaceListProjects(),
	});
	const listing = query.data ?? EMPTY_LISTING;
	return {
		projects: listing.projects,
		unreadableProjects: listing.unreadableProjects,
		loading: query.isPending,
		error: query.error,
	};
}
