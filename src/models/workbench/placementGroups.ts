import type { ProjectView, WorkbenchEntryView } from "@/bindings";

/**
 * The loaded projects split into the three placement groups, each in the
 * Workbench's entry order.
 */
export type PlacementGroups = {
	pinned: ProjectView[];
	shown: ProjectView[];
	hidden: ProjectView[];
};

/**
 * Maps the ordered Workbench entries onto the loaded projects, grouped by
 * placement and keeping the entries' order. Entries whose project is not
 * loaded are dropped: the server reconciles the arrangement against the live
 * project list, so this is only belt-and-suspenders for a project that hasn't
 * loaded yet.
 */
export function groupProjectsByPlacement(
	entries: WorkbenchEntryView[],
	projects: ProjectView[],
): PlacementGroups {
	const projectsByTag = new Map(projects.map((project) => [project.tag, project]));
	const groups: PlacementGroups = { pinned: [], shown: [], hidden: [] };

	for (const entry of entries) {
		const project = projectsByTag.get(entry.projectTag);
		if (!project) continue;
		groups[entry.placement].push(project);
	}

	return groups;
}

/** The projects that render as tabs: pinned first, then shown, in order. */
export function visibleProjects(groups: PlacementGroups): ProjectView[] {
	return [...groups.pinned, ...groups.shown];
}

/** Whether a tag belongs to a project that renders as a tab. */
export function isVisibleTag(groups: PlacementGroups, projectTag: string): boolean {
	return visibleProjects(groups).some((project) => project.tag === projectTag);
}

/**
 * The tab the bar opens on: the last-open project when it is still visible,
 * else the first pinned, else the first shown, else none.
 */
export function initialVisibleTag(
	groups: PlacementGroups,
	lastProjectTag: string | null,
): string | null {
	if (lastProjectTag && isVisibleTag(groups, lastProjectTag)) return lastProjectTag;
	return visibleProjects(groups)[0]?.tag ?? null;
}
