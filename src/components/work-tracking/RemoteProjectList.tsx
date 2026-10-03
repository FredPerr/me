import { ActionIcon, Badge, Group, NavLink, Stack, Title } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import type { WorkProjectId } from "@/domain/work-tracking/identifiers";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { isOpenableProviderUrl } from "@/domain/work-tracking/providerUrl";
import type { WorkProject } from "@/domain/work-tracking/WorkProject";
import { WorkProjectStatus } from "@/domain/work-tracking/WorkProjectStatus";
import type { PagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";
import { LoadMoreButton } from "./LoadMoreButton";
import { RemoteListState } from "./RemoteListState";

const PROJECT_STATUS_LABEL_KEYS: Record<WorkProjectStatus, string> = {
	[WorkProjectStatus.Active]: "workTracking.projects.status.active",
	[WorkProjectStatus.Archived]: "workTracking.projects.status.archived",
	[WorkProjectStatus.Other]: "workTracking.projects.status.other",
};

const LIST_STYLE = { listStyle: "none" } as const;

type RemoteProjectListProps = {
	list: PagedRemoteList<WorkProject>;
	selectedRemoteProjectId: WorkProjectId | null;
	onSelect: (remoteProjectId: WorkProjectId) => void;
	connection: ProviderConnection;
	onReplaceKey: () => void;
};

export function RemoteProjectList({
	list,
	selectedRemoteProjectId,
	onSelect,
	connection,
	onReplaceKey,
}: RemoteProjectListProps) {
	const { t } = useTranslation();

	return (
		<Stack gap="xs">
			<Title order={3}>{t("workTracking.projects.title")}</Title>
			<RemoteListState
				status={list.status}
				error={list.error}
				itemCount={list.items.length}
				emptyText={t("workTracking.projects.empty")}
				onRetry={list.retry}
				onReplaceKey={onReplaceKey}
			>
				<Stack component="ul" gap={2} p={0} m={0} style={LIST_STYLE}>
					{list.items.map((remoteProject) => (
						<RemoteProjectRow
							key={remoteProject.id}
							remoteProject={remoteProject}
							selected={remoteProject.id === selectedRemoteProjectId}
							onSelect={onSelect}
							connection={connection}
						/>
					))}
				</Stack>
				<LoadMoreButton list={list} />
			</RemoteListState>
		</Stack>
	);
}

type RemoteProjectRowProps = {
	remoteProject: WorkProject;
	selected: boolean;
	onSelect: (remoteProjectId: WorkProjectId) => void;
	connection: ProviderConnection;
};

function RemoteProjectRow({
	remoteProject,
	selected,
	onSelect,
	connection,
}: RemoteProjectRowProps) {
	const { t } = useTranslation();
	const canOpen = isOpenableProviderUrl(remoteProject.url, connection.baseUrl);

	return (
		<Group component="li" wrap="nowrap" gap="xs">
			<NavLink
				component="button"
				type="button"
				active={selected}
				aria-current={selected ? "true" : undefined}
				label={remoteProject.name}
				rightSection={
					remoteProject.status !== WorkProjectStatus.Active && (
						<Badge variant="light" color="gray" size="sm">
							{t(PROJECT_STATUS_LABEL_KEYS[remoteProject.status])}
						</Badge>
					)
				}
				onClick={() => onSelect(remoteProject.id)}
				style={{ flex: 1 }}
			/>
			{canOpen && (
				<ActionIcon
					variant="subtle"
					aria-label={t("workTracking.projects.openInProvider", { name: remoteProject.name })}
					onClick={() => openUrl(remoteProject.url)}
				>
					<ArrowSquareOutIcon size={16} />
				</ActionIcon>
			)}
		</Group>
	);
}
