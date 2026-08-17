import { Stack, Text } from "@mantine/core";
import type { Project } from "@/models/Project";
import { ProjectListItem } from "./ProjectListItem";

type ProjectListProps = {
	projects: Project[];
	onEdit: (project: Project) => void;
	onDelete: (project: Project) => void;
};

export function ProjectList({ projects, onEdit, onDelete }: ProjectListProps) {
	if (projects.length === 0) {
		return <Text c="dimmed">No projects configured yet.</Text>;
	}

	return (
		<Stack gap="sm">
			{projects.map((project) => (
				<ProjectListItem
					key={project.tag}
					project={project}
					onEdit={onEdit}
					onDelete={onDelete}
				/>
			))}
		</Stack>
	);
}
