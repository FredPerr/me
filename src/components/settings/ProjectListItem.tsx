import { ActionIcon, Group, Text, Card } from "@mantine/core";
import { PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import type { Project } from "@/models/Project";

type ProjectListItemProps = {
	project: Project;
	onEdit: (project: Project) => void;
	onDelete: (project: Project) => void;
};

export function ProjectListItem({ project, onEdit, onDelete }: ProjectListItemProps) {
	return (
		<Card withBorder padding="sm">
			<Group justify="space-between">
				<div>
					<Text fw={500}>{project.name}</Text>
					<Text size="sm" c="dimmed">
						{project.path || "No path configured"}
					</Text>
				</div>
				<Group gap="xs">
					<ActionIcon variant="subtle" onClick={() => onEdit(project)} aria-label="Edit project">
						<PencilSimpleIcon />
					</ActionIcon>
					<ActionIcon
						variant="subtle"
						color="red"
						onClick={() => onDelete(project)}
						aria-label="Delete project"
					>
						<TrashIcon />
					</ActionIcon>
				</Group>
			</Group>
		</Card>
	);
}
