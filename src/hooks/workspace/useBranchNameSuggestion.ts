import { useMutation, useQuery } from "@tanstack/react-query";
import { commands, type ProjectView } from "@/bindings";
import { branchNameSuggestionQueryKey } from "./workspaceQueryKeys";

const NO_SUGGESTION = "";

/** The project's branch name for `title`, or an empty string while there is none. */
export function useBranchNameSuggestion(projectTag: string, title: string, prefix: string | null) {
	const trimmedTitle = title.trim();
	const query = useQuery({
		queryKey: branchNameSuggestionQueryKey(projectTag, trimmedTitle, prefix),
		queryFn: () => commands.workspaceSuggestBranchName(projectTag, trimmedTitle, prefix),
		staleTime: Number.POSITIVE_INFINITY,
		enabled: trimmedTitle !== "",
	});
	if (trimmedTitle === "" || query.isError) return NO_SUGGESTION;
	return query.data ?? NO_SUGGESTION;
}

type BranchNameRequest = { title: string; prefix?: string | null };

/** Asks for the branch name of a title, under the project's default prefix unless told otherwise. */
export function useSuggestBranchName(project: ProjectView) {
	return useMutation({
		mutationFn: ({ title, prefix = project.branchNaming.defaultPrefix }: BranchNameRequest) =>
			commands.workspaceSuggestBranchName(project.tag, title.trim(), prefix),
	});
}
