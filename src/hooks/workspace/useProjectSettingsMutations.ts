import { useMutation, useQueryClient } from "@tanstack/react-query";
import { commands, type ProjectSettingsInput } from "@/bindings";
import { forgetProject } from "./projectCache";
import { useProjectMutation } from "./useProjectMutation";

type ProjectSettingsChange = { projectTag: string; settings: ProjectSettingsInput };

export function useAddProject() {
	return useProjectMutation(({ projectTag, settings }: ProjectSettingsChange) =>
		commands.workspaceAddProject(projectTag, settings),
	);
}

export function useUpdateProjectSettings() {
	return useProjectMutation(({ projectTag, settings }: ProjectSettingsChange) =>
		commands.workspaceUpdateProjectSettings(projectTag, settings),
	);
}

export function useDeleteProject() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (projectTag: string) => commands.workspaceDeleteProject(projectTag),
		onSuccess: (_nothing, projectTag) => forgetProject(queryClient, projectTag),
	});
}
