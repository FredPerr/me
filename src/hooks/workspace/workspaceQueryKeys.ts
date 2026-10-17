const WORKSPACE_QUERY_ROOT = "workspace";

export const PROJECT_LIST_QUERY_KEY = [WORKSPACE_QUERY_ROOT, "projects"] as const;

export const WORKBENCH_QUERY_KEY = [WORKSPACE_QUERY_ROOT, "workbench"] as const;

export function projectQueryKey(projectTag: string) {
	return [WORKSPACE_QUERY_ROOT, "project", projectTag] as const;
}

export function branchNameSuggestionsQueryKey(projectTag: string) {
	return [WORKSPACE_QUERY_ROOT, "branchNameSuggestion", projectTag] as const;
}

export function branchNameSuggestionQueryKey(
	projectTag: string,
	title: string,
	prefix: string | null,
) {
	return [...branchNameSuggestionsQueryKey(projectTag), title, prefix] as const;
}
