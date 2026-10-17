import { ActionIcon, Button, Group, Stack, TextInput, Title } from "@mantine/core";
import { PlusIcon, XIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProjectView } from "@/bindings";
import { UnreadableProjectsAlert } from "@/components/shared/UnreadableProjectsAlert";
import { useDeleteProject } from "@/hooks/workspace/useProjectSettingsMutations";
import { useProjects } from "@/hooks/workspace/useProjects";
import { useProjectEditor } from "./ProjectEditProvider";
import { ProjectList } from "./ProjectList";

export function ProjectSettings() {
	const { t } = useTranslation();
	const { projects, unreadableProjects } = useProjects();
	const deleteProject = useDeleteProject();
	const { openProjectEditor, openProjectCreator } = useProjectEditor();
	const [searchFilter, setSearchFilter] = useState("");

	function handleDelete(project: ProjectView) {
		deleteProject.mutate(project.tag);
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
					<Button leftSection={<PlusIcon />} onClick={openProjectCreator}>
						{t("settings.projects.addProject")}
					</Button>
				</Group>
			</Group>
			<UnreadableProjectsAlert unreadableProjects={unreadableProjects} />
			<ProjectList
				projects={projects}
				onEdit={openProjectEditor}
				onDelete={handleDelete}
				searchValue={searchFilter}
			/>
		</Stack>
	);
}
