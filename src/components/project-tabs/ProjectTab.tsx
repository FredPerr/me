import { Avatar, TabsTab, Tooltip } from "@mantine/core";
import type { ProjectView } from "@/bindings";
import { getIconComponent } from "@/components/shared/IconPicker";
import { initialsOf } from "@/components/shared/initials";

const TAB_ICON_SIZE = 16;

type ProjectTabProps = {
	project: ProjectView;
};

/** A normal tab: the project's icon when set, then its name. */
export function ProjectTab({ project }: ProjectTabProps) {
	const Icon = project.icon ? getIconComponent(project.icon) : null;

	return (
		<TabsTab
			value={project.tag}
			fw={700}
			leftSection={Icon ? <Icon size={TAB_ICON_SIZE} /> : undefined}
		>
			{project.name}
		</TabsTab>
	);
}

/** A pinned tab: icon-only (or an initials badge), with the name in a tooltip. */
export function PinnedProjectTab({ project }: ProjectTabProps) {
	const Icon = project.icon ? getIconComponent(project.icon) : null;

	return (
		<Tooltip label={project.name}>
			<TabsTab value={project.tag} aria-label={project.name}>
				{Icon ? (
					<Icon size={TAB_ICON_SIZE} />
				) : (
					<Avatar size={TAB_ICON_SIZE + 6} radius="xl">
						{initialsOf(project.name)}
					</Avatar>
				)}
			</TabsTab>
		</Tooltip>
	);
}
