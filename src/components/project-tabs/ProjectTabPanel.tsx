import { Button, Divider, Group, Stack, TabsPanel } from "@mantine/core";
import { AppWindowIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ContextsGrid } from "@/components/contexts/ContextsGrid";
import { SearchContextInput } from "@/components/contexts/SearchContextInput";
import { GitRemoteLink } from "@/components/shared/GitRemoteLink";
import { useContexts } from "@/hooks/useContexts";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Project } from "@/models/Project";

type ProjectTabPanelProps = {
	project: Project;
};

export function ProjectTabPanel({ project }: ProjectTabPanelProps) {
	const { t } = useTranslation();
	const { open, isAvailable } = useOpenInIde();
	const { contexts, createContext, deleteContext } = useContexts(project);
	const [searchFilter, setSearchFilter] = useState("");

	return (
		<TabsPanel value={project.tag}>
			<Stack gap="sm" py="md">
				<Group justify="space-between">
					<SearchContextInput value={searchFilter} onChange={setSearchFilter} />
					<Group gap="xs">
						<GitRemoteLink path={project.path} />
						<Button
							leftSection={<AppWindowIcon size={16} />}
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
		</TabsPanel> // group under a tag for features
	);
}
