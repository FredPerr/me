import { ActionIcon, Button, Group, Modal, Stack, TextInput, Title } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { PlusIcon, XIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProjectSettingsInput, ProjectView } from "@/bindings";
import { UnreadableProjectsAlert } from "@/components/shared/UnreadableProjectsAlert";
import {
	useAddProject,
	useDeleteProject,
	useUpdateProjectSettings,
} from "@/hooks/workspace/useProjectSettingsMutations";
import { useProjects } from "@/hooks/workspace/useProjects";
import { ProjectForm } from "./ProjectForm";
import { ProjectList } from "./ProjectList";

export function ProjectSettings() {
	const { t } = useTranslation();
	const { projects, unreadableProjects, unreadableFiles } = useProjects();
	const addProject = useAddProject();
	const updateProjectSettings = useUpdateProjectSettings();
	const deleteProject = useDeleteProject();
	const [editingProject, setEditingProject] = useState<ProjectView | undefined>();
	const [opened, { open, close }] = useDisclosure(false);
	const [searchFilter, setSearchFilter] = useState("");

	function handleAdd() {
		setEditingProject(undefined);
		open();
	}

	function handleEdit(project: ProjectView) {
		setEditingProject(project);
		open();
	}

	function handleDelete(project: ProjectView) {
		deleteProject.mutate(project.tag);
	}

	function handleSubmit(projectTag: string, settings: ProjectSettingsInput) {
		const saveProject = editingProject ? updateProjectSettings : addProject;
		saveProject.mutate({ projectTag, settings }, { onSuccess: close });
	}

	return (
		<Stack gap="lg">
			<Group justify="space-between">
				<Title order={3}>{t("settings.projects.title")}</Title>
				<Group>
					<TextInput
						value={searchFilter}
						placeholder={t("settings.projects.searchPlaceholder")}
						onChange={(e) => {
							setSearchFilter(e.currentTarget.value);
						}}
						rightSection={
							<ActionIcon
								variant="transparent"
								onClick={() => {
									setSearchFilter("");
								}}
							>
								<XIcon size={10} />
							</ActionIcon>
						}
					/>
					<Button leftSection={<PlusIcon />} onClick={handleAdd}>
						{t("settings.projects.addProject")}
					</Button>
				</Group>
			</Group>
			<UnreadableProjectsAlert
				unreadableProjects={unreadableProjects}
				unreadableFiles={unreadableFiles}
			/>
			<ProjectList
				projects={projects}
				onEdit={handleEdit}
				onDelete={handleDelete}
				searchValue={searchFilter}
			/>
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
		</Stack>
	);
}
