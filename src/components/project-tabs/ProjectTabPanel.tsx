import {
	ActionIcon,
	Badge,
	Button,
	Divider,
	Group,
	Stack,
	TabsPanel,
	Text,
	Title,
	Tooltip,
} from "@mantine/core";
import { ArrowSquareOutIcon, ArrowsClockwiseIcon, GlobeIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ContextsGrid } from "@/components/contexts/ContextsGrid";
import { SearchContextInput } from "@/components/contexts/SearchContextInput";
import { GitRemoteLink } from "@/components/shared/GitRemoteLink";
import { useActiveWorkspaces } from "@/hooks/useActiveWorkspaces";
import { useAppSettings } from "@/hooks/useAppSettings";
import { useContexts } from "@/hooks/useContexts";
import { useListeningPorts } from "@/hooks/useListeningPorts";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Project } from "@/models/Project";

type ProjectTabPanelProps = {
	project: Project;
};

export function ProjectTabPanel({ project }: ProjectTabPanelProps) {
	const { t } = useTranslation();
	const { open, isAvailable } = useOpenInIde();
	const { refresh: refreshWorkspaces } = useActiveWorkspaces();
	const { settings } = useAppSettings();
	const { contexts, createContext, deleteContext } = useContexts(project);
	const { ports } = useListeningPorts(project, settings?.ignoredPorts ?? []);
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
				{ports.length > 0 && (
					<>
						<Divider />
						<Title order={5}>{t("project.ports")}</Title>
						<Group gap="xs" wrap="wrap">
							{ports.map((p) => (
								<Tooltip
									key={p.port}
									label={`${p.processName} (pid ${p.pid})${p.contextName ? ` — ${p.contextName}` : ""}${p.repositoryName ? ` / ${p.repositoryName}` : ""}`}
								>
									<Badge
										size="lg"
										variant="light"
										leftSection={<GlobeIcon size={14} />}
										style={{ cursor: "pointer" }}
										onClick={() => openUrl(`http://localhost:${p.port}`)}
									>
										:{p.port}
										{p.contextName && (
											<Text span size="xs" c="dimmed" ml={4}>
												{p.contextName}
												{p.repositoryName ? ` / ${p.repositoryName}` : ""}
											</Text>
										)}
									</Badge>
								</Tooltip>
							))}
						</Group>
					</>
				)}
			</Stack>
		</TabsPanel>
	);
}
