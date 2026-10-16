import { Tabs, TabsList } from "@mantine/core";
import type { ProjectView } from "@/bindings";
import { ProjectTab } from "./ProjectTab";
import { ProjectTabPanel } from "./ProjectTabPanel";

type ProjectTabsProps = {
	projects: ProjectView[];
};

export function ProjectTabs({ projects }: ProjectTabsProps) {
	const firstProject = projects[0];

	return (
		<Tabs
			defaultValue={firstProject.tag}
			style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
		>
			<TabsList>
				{projects.map((project) => (
					<ProjectTab
						displayName={project.name}
						projectTag={project.tag}
						icon={project.icon ?? undefined}
						key={project.tag}
					/>
				))}
			</TabsList>
			{projects.map((project) => (
				<ProjectTabPanel projectTag={project.tag} key={project.tag} />
			))}
		</Tabs>
	);
}
