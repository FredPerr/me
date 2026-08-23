import { ActionIcon, Card, Group, Text } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { Project } from "@/models/Project";
import { ProjectConfirmDeleteModal } from "./ProjectConfirmDeleteModal";

type ProjectListItemProps = {
	project: Project;
	onEdit: (project: Project) => void;
	onDelete: (project: Project) => void;
};

export function ProjectListItem({ project, onEdit, onDelete }: ProjectListItemProps) {
	const { t } = useTranslation();
	const [opened, { open, close }] = useDisclosure(false);
	return (
		<Card withBorder padding="sm">
			<Group justify="space-between">
				<div>
					<Text fw={500}>{project.name}</Text>
					<Text size="sm" c="dimmed">
						{project.path}
					</Text>
					<Text>{t("settings.projects.repoCount", { count: project.repositories.length })}</Text>
				</div>
				<Group gap="xs">
					<ActionIcon variant="subtle" onClick={() => onEdit(project)} aria-label="Edit project">
						<PencilSimpleIcon />
					</ActionIcon>
					<ActionIcon
						variant="subtle"
						color="red"
						onClick={() => {
							open();
						}}
						aria-label="Delete project"
					>
						<TrashIcon />
					</ActionIcon>
					<ProjectConfirmDeleteModal
						project={project}
						onDelete={onDelete}
						onCancel={close}
						opened={opened}
					/>
				</Group>
			</Group>
		</Card>
	);
}
