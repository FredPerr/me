import { Tabs, TabsList } from "@mantine/core";
import type { ProjectView } from "@/bindings";
import { useSetLastProject, useWorkbench } from "@/hooks/workspace/useWorkbench";
import {
	groupProjectsByPlacement,
	initialVisibleTag,
	visibleProjects,
} from "@/models/workbench/placementGroups";
import { HiddenProjectsButton } from "./HiddenProjectsButton";
import { PinnedProjectTab, ProjectTab } from "./ProjectTab";
import { ProjectTabPanel } from "./ProjectTabPanel";

type ProjectTabsProps = {
	projects: ProjectView[];
};

export function ProjectTabs({ projects }: ProjectTabsProps) {
	const { workbench } = useWorkbench();
	const setLastProject = useSetLastProject();

	const groups = groupProjectsByPlacement(workbench.entries, projects);
	const activeTag = initialVisibleTag(groups, workbench.lastProjectTag);
	const visible = visibleProjects(groups);

	if (!activeTag) return null;

	function handleChange(value: string | null) {
		if (value) setLastProject.mutate(value);
	}

	return (
		<Tabs
			value={activeTag}
			onChange={handleChange}
			style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
		>
			<TabsList>
				{groups.pinned.map((project) => (
					<PinnedProjectTab project={project} key={project.tag} />
				))}
				{groups.shown.map((project) => (
					<ProjectTab project={project} key={project.tag} />
				))}
				<HiddenProjectsButton hiddenProjects={groups.hidden} />
			</TabsList>
			{visible.map((project) => (
				<ProjectTabPanel projectTag={project.tag} key={project.tag} />
			))}
		</Tabs>
	);
}
