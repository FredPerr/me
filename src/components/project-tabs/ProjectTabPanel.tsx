import { ActionIcon, Button, Divider, Group, Stack, TabsPanel, Tooltip } from "@mantine/core";
import { ArrowSquareOutIcon, ArrowsClockwiseIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ContextsGrid } from "@/components/contexts/ContextsGrid";
import { SearchContextInput } from "@/components/contexts/SearchContextInput";
import { GitRemoteLink } from "@/components/shared/GitRemoteLink";
import { useActiveWorkspaces } from "@/hooks/useActiveWorkspaces";
import { useContexts } from "@/hooks/useContexts";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Project } from "@/models/Project";

type ProjectTabPanelProps = {
	project: Project;
};

export function ProjectTabPanel({ project }: ProjectTabPanelProps) {
	const { t } = useTranslation();
	const { open, isAvailable } = useOpenInIde();
	const { refresh: refreshWorkspaces } = useActiveWorkspaces();
	const { contexts, createContext, deleteContext } = useContexts(project);
	const [searchFilter, setSearchFilter] = useState("");

	function handleRefresh() {
		refreshWorkspaces();
	}

	return (
		<TabsPanel value={project.tag}>
			<Stack gap="sm" py="md">
				<Group justify="space-between">
					<SearchContextInput value={searchFilter} onChange={setSearchFilter} />
					<Group gap="xs">
						<Tooltip label={t("common.refresh")}>
							<ActionIcon variant="subtle" size="sm" onClick={handleRefresh} aria-label="Refresh">
								<ArrowsClockwiseIcon size={16} />
							</ActionIcon>
						</Tooltip>
						<GitRemoteLink path={project.path} />
						<Button
							leftSection={<ArrowSquareOutIcon size={16} />}
							size="xs"
							onClick={() => open(project.path)}
							disabled={!project.path || !isAvailable}
						>
							{t("project.openInIde")}
						</Button>
					</Group>
				</Group>
				<Divider />
				<ContextsGrid
					contexts={contexts}
					project={project}
					onDelete={deleteContext}
					onCreate={createContext}
					searchFilter={searchFilter}
				/>
			</Stack>
		</TabsPanel>
	);
}
