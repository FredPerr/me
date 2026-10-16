import type { QueryClient } from "@tanstack/react-query";
import type { ProjectListView, ProjectView } from "@/bindings";
import {
	branchNameSuggestionsQueryKey,
	PROJECT_LIST_QUERY_KEY,
	projectQueryKey,
} from "./workspaceQueryKeys";

/** Puts the Project a command returned into the project query and the project list. */
export function storeProject(queryClient: QueryClient, project: ProjectView): void {
	queryClient.setQueryData(projectQueryKey(project.tag), project);
	queryClient.setQueryData<ProjectListView>(PROJECT_LIST_QUERY_KEY, (listing) => {
		if (!listing) return listing;
		const isListed = listing.projects.some((listed) => listed.tag === project.tag);
		const projects = isListed
			? listing.projects.map((listed) => (listed.tag === project.tag ? project : listed))
			: [...listing.projects, project];
		return { ...listing, projects };
	});
}

export function forgetProject(queryClient: QueryClient, projectTag: string): void {
	queryClient.removeQueries({ queryKey: projectQueryKey(projectTag) });
	queryClient.setQueryData<ProjectListView>(PROJECT_LIST_QUERY_KEY, (listing) =>
		listing
			? { ...listing, projects: listing.projects.filter((listed) => listed.tag !== projectTag) }
			: listing,
	);
}

export async function invalidateProject(queryClient: QueryClient, projectTag: string) {
	await Promise.all([
		queryClient.invalidateQueries({ queryKey: projectQueryKey(projectTag) }),
		queryClient.invalidateQueries({ queryKey: PROJECT_LIST_QUERY_KEY }),
		queryClient.invalidateQueries({ queryKey: branchNameSuggestionsQueryKey(projectTag) }),
	]);
}
