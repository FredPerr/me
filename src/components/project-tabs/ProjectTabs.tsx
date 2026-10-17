import { DndContext, type DragEndEvent, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import { horizontalListSortingStrategy, SortableContext } from "@dnd-kit/sortable";
import { Tabs, TabsList } from "@mantine/core";
import type { ProjectView } from "@/bindings";
import { useReorder, useSetLastProject, useWorkbench } from "@/hooks/workspace/useWorkbench";
import {
	groupProjectsByPlacement,
	initialVisibleTag,
	type PlacementGroups,
	visibleProjects,
} from "@/models/workbench/placementGroups";
import { decideReorder } from "@/models/workbench/reorder";
import { HiddenProjectsButton } from "./HiddenProjectsButton";
import { PinnedProjectTab, ProjectTab } from "./ProjectTab";
import { ProjectTabMenu } from "./ProjectTabMenu";
import { ProjectTabPanel } from "./ProjectTabPanel";
import { SortableProjectTab } from "./SortableProjectTab";

type ProjectTabsProps = {
	projects: ProjectView[];
};

/** Below this drag distance a pointer gesture stays a click, so tabs still select. */
const DRAG_ACTIVATION_DISTANCE = 5;

export function ProjectTabs({ projects }: ProjectTabsProps) {
	const { workbench } = useWorkbench();
	const setLastProject = useSetLastProject();
	const reorder = useReorder();
	const sensors = useSensors(
		useSensor(PointerSensor, { activationConstraint: { distance: DRAG_ACTIVATION_DISTANCE } }),
	);

	const groups = groupProjectsByPlacement(workbench.entries, projects);
	const activeTag = initialVisibleTag(groups, workbench.lastProjectTag);
	const visible = visibleProjects(groups);

	if (!activeTag) return null;

	function handleChange(value: string | null) {
		if (value) setLastProject.mutate(value);
	}

	function handleDragEnd(event: DragEndEvent) {
		const activeTag = String(event.active.id);
		const overTag = event.over ? String(event.over.id) : null;
		const decision = decideReorder(groups, activeTag, overTag);
		if (decision) reorder.mutate(decision);
	}

	return (
		<Tabs
			value={activeTag}
			onChange={handleChange}
			style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
		>
			<DndContext sensors={sensors} onDragEnd={handleDragEnd}>
				<TabsList>
					<SortableGroup tags={groupTags(groups.pinned)}>
						{groups.pinned.map((project) => (
							<SortableProjectTab projectTag={project.tag} key={project.tag}>
								<ProjectTabMenu project={project} pinned>
									<PinnedProjectTab project={project} />
								</ProjectTabMenu>
							</SortableProjectTab>
						))}
					</SortableGroup>
					<SortableGroup tags={groupTags(groups.shown)}>
						{groups.shown.map((project) => (
							<SortableProjectTab projectTag={project.tag} key={project.tag}>
								<ProjectTabMenu project={project} pinned={false}>
									<ProjectTab project={project} />
								</ProjectTabMenu>
							</SortableProjectTab>
						))}
					</SortableGroup>
					<HiddenProjectsButton hiddenProjects={groups.hidden} />
				</TabsList>
			</DndContext>
			{visible.map((project) => (
				<ProjectTabPanel projectTag={project.tag} key={project.tag} />
			))}
		</Tabs>
	);
}

function groupTags(projects: PlacementGroups["pinned"]): string[] {
	return projects.map((project) => project.tag);
}

/** One sortable list so a group reorders only among itself. */
function SortableGroup({ tags, children }: { tags: string[]; children: React.ReactNode }) {
	return (
		<SortableContext items={tags} strategy={horizontalListSortingStrategy}>
			{children}
		</SortableContext>
	);
}
