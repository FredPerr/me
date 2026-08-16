import { TabsPanel, Text } from "@mantine/core";
import type { Project } from "@/models/Project";

type ProjectTabPanelProps = {
	project: Project;
};

export function ProjectTabPanel({ project }: ProjectTabPanelProps) {
	return (
		<TabsPanel value={project.tag}>
			<Text>{project.name}</Text>
		</TabsPanel>
	);
}
