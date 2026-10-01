import { ActionIcon, Badge, Group, NavLink, Stack, Title, Tooltip } from "@mantine/core";
import { ArrowSquareOutIcon, LinkIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import type { WorkProjectId } from "@/domain/work-tracking/identifiers";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { isOpenableProviderUrl } from "@/domain/work-tracking/providerUrl";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import type { WorkProject } from "@/domain/work-tracking/WorkProject";
import { WorkProjectStatus } from "@/domain/work-tracking/WorkProjectStatus";
import type { PagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";
import type { RemoteProjectLinkActions } from "@/hooks/work-tracking/useRemoteProjectLinks";
import type { Project } from "@/models/Project";
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
	selectedProject: Project | null;
	connection: ProviderConnection;
	link: RemoteProjectLinkActions["link"];
	unlink: RemoteProjectLinkActions["unlink"];
	isPending: RemoteProjectLinkActions["isPending"];
	onReplaceKey: () => void;
};

export function RemoteProjectList({
	list,
	selectedRemoteProjectId,
	onSelect,
	selectedProject,
	connection,
	link,
	unlink,
	isPending,
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
							selectedProject={selectedProject}
							connection={connection}
							link={link}
							unlink={unlink}
							isPending={isPending}
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
	selectedProject: Project | null;
	connection: ProviderConnection;
	link: RemoteProjectLinkActions["link"];
	unlink: RemoteProjectLinkActions["unlink"];
	isPending: RemoteProjectLinkActions["isPending"];
};

function RemoteProjectRow({
	remoteProject,
	selected,
	onSelect,
	selectedProject,
	connection,
	link,
	unlink,
	isPending,
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
			<LinkToggle
				remoteProject={remoteProject}
				selectedProject={selectedProject}
				connection={connection}
				link={link}
				unlink={unlink}
				isPending={isPending}
			/>
		</Group>
	);
}

type LinkToggleProps = {
	remoteProject: WorkProject;
	selectedProject: Project | null;
	connection: ProviderConnection;
	link: RemoteProjectLinkActions["link"];
	unlink: RemoteProjectLinkActions["unlink"];
	isPending: RemoteProjectLinkActions["isPending"];
};

function LinkToggle({
	remoteProject,
	selectedProject,
	connection,
	link,
	unlink,
	isPending,
}: LinkToggleProps) {
	const { t } = useTranslation();

	if (!selectedProject) {
		const hint = t("workTracking.link.selectLocalProjectFirst");
		// data-disabled instead of disabled keeps hover and focus events so the Tooltip still shows.
		return (
			<Tooltip label={hint}>
				<ActionIcon
					variant="subtle"
					color="gray"
					data-disabled
					aria-disabled="true"
					aria-label={hint}
					onClick={(event) => event.preventDefault()}
				>
					<LinkIcon size={16} />
				</ActionIcon>
			</Tooltip>
		);
	}

	const identityKey = RemoteProjectLink.identityKeyOf(
		connection.kind,
		connection.id,
		remoteProject.id,
	);
	const existingLink = selectedProject.remoteProjectLinks
		.toArray()
		.find((candidate) => candidate.identityKey() === identityKey);
	const isLinked = existingLink !== undefined;
	const pending = isPending(selectedProject.tag, identityKey);

	function handleToggle() {
		if (!selectedProject || pending) return;
		if (existingLink) unlink(selectedProject, existingLink);
		else link(selectedProject, remoteProject, connection);
	}

	return (
		<ActionIcon
			variant={isLinked ? "filled" : "subtle"}
			aria-label={t("workTracking.link.linkTo", {
				name: remoteProject.name,
				project: selectedProject.name,
			})}
			aria-pressed={isLinked}
			aria-busy={pending ? "true" : undefined}
			disabled={pending}
			onClick={handleToggle}
		>
			<LinkIcon size={16} weight={isLinked ? "bold" : "regular"} />
		</ActionIcon>
	);
}
