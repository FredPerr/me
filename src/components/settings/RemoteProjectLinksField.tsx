import { ActionIcon, Badge, Group, Input, Select, Stack, Text, Tooltip } from "@mantine/core";
import { ArrowSquareOutIcon, LinkBreakIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { LoadMoreButton } from "@/components/work-tracking/LoadMoreButton";
import { translateError } from "@/components/work-tracking/translateError";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { DEFAULT_PROVIDER_KIND } from "@/domain/work-tracking/ProviderKind";
import { isOpenableProviderUrl } from "@/domain/work-tracking/providerUrl";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import type { RemoteProjectLinks } from "@/domain/work-tracking/RemoteProjectLinks";
import type { WorkProject } from "@/domain/work-tracking/WorkProject";
import { PagedListStatus } from "@/hooks/work-tracking/pagedListState";
import { usePagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";
import { useWorkTrackingConnections } from "@/hooks/work-tracking/useWorkTrackingConnections";
import { workTrackingGateway } from "@/infra/work-tracking/workTracking";

type RemoteProjectLinksFieldProps = {
	value: RemoteProjectLinks;
	onChange: (links: RemoteProjectLinks) => void;
};

export function RemoteProjectLinksField({ value, onChange }: RemoteProjectLinksFieldProps) {
	const { t } = useTranslation();
	const { connections, revision } = useWorkTrackingConnections();
	const connection =
		connections.find((candidate) => candidate.kind === DEFAULT_PROVIDER_KIND) ?? null;
	const links = value.toArray();

	return (
		<Stack gap="xs">
			<div>
				<Input.Label>{t("settings.projectForm.remoteProjects")}</Input.Label>
				<Input.Description>{t("settings.projectForm.remoteProjectsDescription")}</Input.Description>
			</div>
			{links.length === 0 ? (
				<Text size="xs" c="dimmed">
					{t("workTracking.link.noLinks")}
				</Text>
			) : (
				<Stack component="ul" gap={4} p={0} m={0} style={{ listStyle: "none" }}>
					{links.map((link) => (
						<LinkedRemoteProjectRow
							key={link.identityKey()}
							link={link}
							linkConnection={connections.find((candidate) => candidate.id === link.connectionId)}
							onUnlink={() => onChange(value.remove(link.identityKey()))}
						/>
					))}
				</Stack>
			)}
			{connection ? (
				<RemoteProjectPicker
					connection={connection}
					revision={revision}
					links={value}
					onLink={(link) => onChange(value.add(link))}
				/>
			) : (
				<Text size="xs" c="dimmed">
					{t("settings.projectForm.remoteProjectsNoConnection")}
				</Text>
			)}
		</Stack>
	);
}

type RemoteProjectPickerProps = {
	connection: ProviderConnection;
	revision: number;
	links: RemoteProjectLinks;
	onLink: (link: RemoteProjectLink) => void;
};

function RemoteProjectPicker({ connection, revision, links, onLink }: RemoteProjectPickerProps) {
	const { t } = useTranslation();
	const projectList = usePagedRemoteList<WorkProject>(
		(cursor) => workTrackingGateway.listProjects(connection.id, cursor),
		`${revision}|${connection.id}`,
	);

	const unlinkedProjects = useMemo(
		() =>
			projectList.items.filter(
				(remoteProject) => !links.has(connection.kind, connection.id, remoteProject.id),
			),
		[projectList.items, links, connection],
	);

	function handleSelect(remoteProjectId: string | null) {
		const remoteProject = unlinkedProjects.find((candidate) => candidate.id === remoteProjectId);
		if (!remoteProject) return;
		onLink(
			RemoteProjectLink.create(
				{
					providerKind: connection.kind,
					connectionId: connection.id,
					remoteProjectId: remoteProject.id,
					remoteProjectName: remoteProject.name,
					remoteProjectUrl: remoteProject.url,
				},
				new Date(),
			),
		);
	}

	return (
		<Group gap="xs" align="flex-end">
			<Select
				size="xs"
				style={{ flex: 1 }}
				placeholder={t("settings.projectForm.linkRemoteProject", {
					provider: t(`workTracking.providers.${connection.kind}`),
				})}
				data={unlinkedProjects.map((remoteProject) => ({
					value: remoteProject.id,
					label: remoteProject.name,
				}))}
				value={null}
				onChange={handleSelect}
				disabled={projectList.status === PagedListStatus.Loading}
				error={projectList.error ? translateError(t, projectList.error) : undefined}
				nothingFoundMessage={t("workTracking.projects.empty")}
				searchable
			/>
			<LoadMoreButton list={projectList} />
		</Group>
	);
}

type LinkedRemoteProjectRowProps = {
	link: RemoteProjectLink;
	linkConnection: ProviderConnection | undefined;
	onUnlink: () => void;
};

function LinkedRemoteProjectRow({ link, linkConnection, onUnlink }: LinkedRemoteProjectRowProps) {
	const { t } = useTranslation();
	const openableUrl =
		linkConnection &&
		link.remoteProjectUrl !== undefined &&
		isOpenableProviderUrl(link.remoteProjectUrl, linkConnection.baseUrl)
			? link.remoteProjectUrl
			: null;

	return (
		<Group component="li" justify="space-between" wrap="nowrap" gap="sm">
			<Group gap="xs" wrap="nowrap" style={{ minWidth: 0 }}>
				<Text size="sm" truncate>
					{link.remoteProjectName}
				</Text>
				<Badge variant="light" size="sm">
					{t(`workTracking.providers.${link.providerKind}`)}
				</Badge>
				{!linkConnection && (
					<Badge variant="light" color="yellow" size="sm">
						{t("workTracking.link.connectionUnavailable")}
					</Badge>
				)}
			</Group>
			<Group gap={4} wrap="nowrap">
				{openableUrl && (
					<Tooltip label={t("workTracking.link.open")}>
						<ActionIcon
							variant="subtle"
							size="sm"
							onClick={() => openUrl(openableUrl)}
							aria-label={t("workTracking.projects.openInProvider", {
								name: link.remoteProjectName,
							})}
						>
							<ArrowSquareOutIcon size={14} />
						</ActionIcon>
					</Tooltip>
				)}
				<Tooltip label={t("workTracking.link.unlink")}>
					<ActionIcon
						variant="subtle"
						color="red"
						size="sm"
						onClick={onUnlink}
						aria-label={t("workTracking.link.unlinkNamed", { name: link.remoteProjectName })}
					>
						<LinkBreakIcon size={14} />
					</ActionIcon>
				</Tooltip>
			</Group>
		</Group>
	);
}
