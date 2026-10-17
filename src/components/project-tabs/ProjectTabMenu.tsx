import { Menu } from "@mantine/core";
import { EyeSlashIcon, GearSixIcon, PushPinIcon, PushPinSlashIcon } from "@phosphor-icons/react";
import type { ReactNode } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { PlacementView, ProjectView } from "@/bindings";
import { useProjectEditor } from "@/components/settings/ProjectEditProvider";
import { useSetPlacement } from "@/hooks/workspace/useWorkbench";

type ProjectTabMenuProps = {
	project: ProjectView;
	/** Whether the tab is pinned, so the menu offers Unpin instead of Pin. */
	pinned: boolean;
	children: ReactNode;
};

/**
 * Wraps a tab with a right-click menu to pin/unpin, hide, and open the project's
 * settings. Mantine's Menu has no native context-menu trigger, so a controlled
 * Menu is opened from `onContextMenu`; the tab trigger stays presentational.
 */
export function ProjectTabMenu({ project, pinned, children }: ProjectTabMenuProps) {
	const { t } = useTranslation();
	const setPlacement = useSetPlacement();
	const { openProjectEditor } = useProjectEditor();
	const [opened, setOpened] = useState(false);

	function changePlacement(placement: PlacementView) {
		setPlacement.mutate({ projectTag: project.tag, placement });
	}

	function openMenu(event: React.MouseEvent) {
		event.preventDefault();
		setOpened(true);
	}

	return (
		<Menu opened={opened} onChange={setOpened} position="bottom-start" withinPortal>
			<Menu.Target>
				{/* The whole tab is the anchor; right-click opens, left-click falls through to select. */}
				{/* biome-ignore lint/a11y/noStaticElementInteractions: a context menu is a right-click-only affordance; the tab inside keeps its own keyboard interaction. */}
				<div onContextMenu={openMenu} style={{ display: "inline-flex" }}>
					{children}
				</div>
			</Menu.Target>
			<Menu.Dropdown>
				{pinned ? (
					<Menu.Item
						leftSection={<PushPinSlashIcon size={16} />}
						onClick={() => changePlacement("shown")}
					>
						{t("workbench.unpin")}
					</Menu.Item>
				) : (
					<Menu.Item
						leftSection={<PushPinIcon size={16} />}
						onClick={() => changePlacement("pinned")}
					>
						{t("workbench.pin")}
					</Menu.Item>
				)}
				<Menu.Item
					leftSection={<EyeSlashIcon size={16} />}
					onClick={() => changePlacement("hidden")}
				>
					{t("workbench.hide")}
				</Menu.Item>
				<Menu.Item
					leftSection={<GearSixIcon size={16} />}
					onClick={() => openProjectEditor(project)}
				>
					{t("workbench.projectSettings")}
				</Menu.Item>
			</Menu.Dropdown>
		</Menu>
	);
}
