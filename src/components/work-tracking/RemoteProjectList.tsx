import { ActionIcon, Badge, Group, NavLink, Stack, Title } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import type {
	ConnectionView,
	RemoteProject,
	RemoteProjectId,
	RemoteProjectStatus,
} from "@/bindings";
import { isOpenableProviderUrl } from "@/hooks/work-tracking/providerUrl";
import type { PagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";
import { LoadMoreButton } from "./LoadMoreButton";
import { RemoteListState } from "./RemoteListState";

const PROJECT_STATUS_LABEL_KEYS: Record<RemoteProjectStatus, string> = {
	active: "workTracking.projects.status.active",
	archived: "workTracking.projects.status.archived",
	other: "workTracking.projects.status.other",
};

const LIST_STYLE = { listStyle: "none" } as const;

type RemoteProjectListProps = {
	list: PagedRemoteList<RemoteProject>;
	selectedRemoteProjectId: RemoteProjectId | null;
	onSelect: (remoteProjectId: RemoteProjectId) => void;
	connection: ConnectionView;
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
	remoteProject: RemoteProject;
	selected: boolean;
	onSelect: (remoteProjectId: RemoteProjectId) => void;
	connection: ConnectionView;
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
					remoteProject.status !== "active" && (
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
