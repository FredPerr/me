import { Button, Menu } from "@mantine/core";
import { EyeIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { ProjectView } from "@/bindings";
import { useSetPlacement } from "@/hooks/workspace/useWorkbench";

type HiddenProjectsButtonProps = {
	hiddenProjects: ProjectView[];
};

/** The "+N" trigger at the end of the bar; each hidden project gets a Show action. */
export function HiddenProjectsButton({ hiddenProjects }: HiddenProjectsButtonProps) {
	const { t } = useTranslation();
	const setPlacement = useSetPlacement();

	if (hiddenProjects.length === 0) return null;

	function show(projectTag: string) {
		setPlacement.mutate({ projectTag, placement: "shown" });
	}

	return (
		<Menu position="bottom-end" withinPortal>
			<Menu.Target>
				<Button variant="subtle" size="xs" aria-label={t("workbench.hiddenProjects")}>
					+{hiddenProjects.length}
				</Button>
			</Menu.Target>
			<Menu.Dropdown>
				<Menu.Label>{t("workbench.hiddenProjects")}</Menu.Label>
				{hiddenProjects.map((project) => (
					<Menu.Item
						key={project.tag}
						leftSection={<EyeIcon size={16} />}
						onClick={() => show(project.tag)}
					>
						{t("workbench.showProject", { name: project.name })}
					</Menu.Item>
				))}
			</Menu.Dropdown>
		</Menu>
	);
}
