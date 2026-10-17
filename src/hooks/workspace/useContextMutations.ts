import { type ContextStatusView, type CreateContextInput, commands } from "@/bindings";
import { useProjectMutation } from "./useProjectMutation";

type ContextStatusChange = { contextId: string; status: ContextStatusView };
type ContextStaticChange = { contextId: string; isStatic: boolean };
type PullRequestDraftChange = {
	contextId: string;
	repositoryId: string;
	title: string;
	description: string;
};
type AgentPromptChange = { contextId: string; prompt: string | null };

export function useCreateContext() {
	return useProjectMutation((input: CreateContextInput) => commands.workspaceCreateContext(input));
}

export function useDeleteContext(projectTag: string) {
	return useProjectMutation((contextId: string) =>
		commands.workspaceDeleteContext(projectTag, contextId),
	);
}

export function useSetContextStatus(projectTag: string) {
	return useProjectMutation(({ contextId, status }: ContextStatusChange) =>
		commands.workspaceSetContextStatus(projectTag, contextId, status),
	);
}

export function useSetContextStatic(projectTag: string) {
	return useProjectMutation(({ contextId, isStatic }: ContextStaticChange) =>
		commands.workspaceSetContextStatic(projectTag, contextId, isStatic),
	);
}

export function useSavePullRequestDraft(projectTag: string) {
	return useProjectMutation(
		({ contextId, repositoryId, title, description }: PullRequestDraftChange) =>
			commands.workspaceSavePullRequestDraft(
				projectTag,
				contextId,
				repositoryId,
				title,
				description,
			),
	);
}

export function useSetAgentPrompt(projectTag: string) {
	return useProjectMutation(({ contextId, prompt }: AgentPromptChange) =>
		commands.workspaceSetAgentPrompt(projectTag, contextId, prompt),
	);
}
