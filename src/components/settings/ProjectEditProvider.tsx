import { Modal } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { createContext, type ReactNode, useContext, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProjectSettingsInput, ProjectView } from "@/bindings";
import {
	useAddProject,
	useUpdateProjectSettings,
} from "@/hooks/workspace/useProjectSettingsMutations";
import { ProjectForm } from "./ProjectForm";

type ProjectEditContextValue = {
	/** Opens the editor for an existing project. */
	openProjectEditor: (project: ProjectView) => void;
	/** Opens the editor to create a new project. */
	openProjectCreator: () => void;
};

const ProjectEditContext = createContext<ProjectEditContextValue | null>(null);

/**
 * A single app-wide project edit/create modal. Both Settings → Projects and the
 * tab right-click menu open it through `useProjectEditor`, so there is one modal
 * instance and one source of add-vs-edit behaviour.
 */
export function ProjectEditProvider({ children }: { children: ReactNode }) {
	const { t } = useTranslation();
	const addProject = useAddProject();
	const updateProjectSettings = useUpdateProjectSettings();
	const [editingProject, setEditingProject] = useState<ProjectView | undefined>();
	const [opened, { open, close }] = useDisclosure(false);

	const value = useMemo<ProjectEditContextValue>(
		() => ({
			openProjectEditor: (project) => {
				setEditingProject(project);
				open();
			},
			openProjectCreator: () => {
				setEditingProject(undefined);
				open();
			},
		}),
		[open],
	);

	function handleSubmit(projectTag: string, settings: ProjectSettingsInput) {
		const saveProject = editingProject ? updateProjectSettings : addProject;
		saveProject.mutate({ projectTag, settings }, { onSuccess: close });
	}

	return (
		<ProjectEditContext.Provider value={value}>
			{children}
			<Modal
				opened={opened}
				onClose={close}
				size="xl"
				title={
					editingProject ? t("settings.projects.editProject") : t("settings.projects.addProject")
				}
			>
				<ProjectForm
					initialProject={editingProject}
					submitting={addProject.isPending || updateProjectSettings.isPending}
					onSubmit={handleSubmit}
					onCancel={close}
				/>
			</Modal>
		</ProjectEditContext.Provider>
	);
}

export function useProjectEditor(): ProjectEditContextValue {
	const value = useContext(ProjectEditContext);
	if (!value) {
		throw new Error("useProjectEditor must be used within a ProjectEditProvider");
	}
	return value;
}
