import { Stack, Text } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { ProjectView } from "@/bindings";
import { ProjectListItem } from "./ProjectListItem";

type ProjectListProps = {
	projects: ProjectView[];
	onEdit: (project: ProjectView) => void;
	onDelete: (project: ProjectView) => void;
	searchValue: string;
};

export function ProjectList({ projects, onEdit, onDelete, searchValue }: ProjectListProps) {
	const { t } = useTranslation();
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
				<Text c="dimmed">{t("settings.projects.noProjects")}</Text>
			)}
		</Stack>
	);
}
