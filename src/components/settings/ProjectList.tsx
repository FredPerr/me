import { Stack, Text } from "@mantine/core";
import type { Project } from "@/models/Project";
import { ProjectListItem } from "./ProjectListItem";

type ProjectListProps = {
	projects: Project[];
	onEdit: (project: Project) => void;
	onDelete: (project: Project) => void;
	searchValue: string
};

export function ProjectList({ projects, onEdit, onDelete, searchValue }: ProjectListProps) {
	const filteredProjects = projects.filter((p) => {
		const combineString = `${p.name}${p.path}${p.tag}`.toLowerCase().trim();
		return combineString.includes(searchValue.toLowerCase());
	});

	return (
		<Stack gap="sm">
			{filteredProjects.length > 0 ? (
				filteredProjects.map((project) => (
					<ProjectListItem
						key={project.tag}
						project={project}
						onEdit={onEdit}
						onDelete={onDelete}
					/>
				))
			) : (
				<Text c="dimmed">No projects configured yet.</Text>
			)}
		</Stack>
	);
}
