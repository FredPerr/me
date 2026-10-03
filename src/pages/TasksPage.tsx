import {
	Alert,
	Anchor,
	Button,
	Grid,
	Group,
	Loader,
	Select,
	Stack,
	Text,
	Title,
	Tooltip,
} from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { SparkleIcon } from "@phosphor-icons/react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { BulkCreateFromTasksModal } from "@/components/work-tracking/BulkCreateFromTasksModal";
import { RemoteProjectList } from "@/components/work-tracking/RemoteProjectList";
import { translateError } from "@/components/work-tracking/translateError";
import { WorkItemGroupList } from "@/components/work-tracking/WorkItemGroupList";
import { WorkItemList } from "@/components/work-tracking/WorkItemList";
import type {
	WorkItemGroupId,
	WorkItemId,
	WorkProjectId,
} from "@/domain/work-tracking/identifiers";
import { DEFAULT_PROVIDER_KIND } from "@/domain/work-tracking/ProviderKind";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import type { WorkItemGroup } from "@/domain/work-tracking/WorkItemGroup";
import type { WorkProject } from "@/domain/work-tracking/WorkProject";
import { useProjects } from "@/hooks/useProjects";
import { usePagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";
import {
	ConnectionsStatus,
	useWorkTrackingConnections,
} from "@/hooks/work-tracking/useWorkTrackingConnections";
import { workTrackingGateway } from "@/infra/work-tracking/workTracking";
import { SettingsTab, settingsPath } from "./settingsTabs";

export function TasksPage() {
	const { t } = useTranslation();
	const navigate = useNavigate();
	const { connections, status, error, revision, reload } = useWorkTrackingConnections();
	const { projects, loading } = useProjects();
	const [selectedProjectTag, setSelectedProjectTag] = useState<string | null>(null);
	const [selectedRemoteProjectId, setSelectedRemoteProjectId] = useState<WorkProjectId | null>(
		null,
	);
	const [selectedGroupId, setSelectedGroupId] = useState<WorkItemGroupId | null>(null);
	const [selectedTaskIds, setSelectedTaskIds] = useState<ReadonlySet<WorkItemId>>(() => new Set());
	const [bulkOpened, { open: openBulk, close: closeBulk }] = useDisclosure(false);

	const selectedProject = projects.find((project) => project.tag === selectedProjectTag) ?? null;
	const connection =
		connections.find((candidate) => candidate.kind === DEFAULT_PROVIDER_KIND) ?? null;
	const connectionId = connection?.id;

	const projectList = usePagedRemoteList<WorkProject>(
		connectionId ? (cursor) => workTrackingGateway.listProjects(connectionId, cursor) : null,
		connectionId ? `${revision}|${connectionId}` : "",
	);
	const groupList = usePagedRemoteList<WorkItemGroup>(
		connectionId && selectedRemoteProjectId
			? (cursor) => workTrackingGateway.listGroups(connectionId, selectedRemoteProjectId, cursor)
			: null,
		connectionId && selectedRemoteProjectId
			? `${revision}|${connectionId}|${selectedRemoteProjectId}`
			: "",
	);
	const itemList = usePagedRemoteList<WorkItem>(
		connectionId && selectedRemoteProjectId && selectedGroupId
			? (cursor) =>
					workTrackingGateway.listItems(
						connectionId,
						selectedRemoteProjectId,
						selectedGroupId,
						cursor,
					)
			: null,
		connectionId && selectedRemoteProjectId && selectedGroupId
			? `${revision}|${connectionId}|${selectedRemoteProjectId}|${selectedGroupId}`
			: "",
	);

	const canSelectTasks = selectedProject !== null;
	const selectedTasks = useMemo(
		() => itemList.items.filter((item) => selectedTaskIds.has(item.id)),
		[itemList.items, selectedTaskIds],
	);

	const toggleTaskSelected = useCallback((item: WorkItem, selected: boolean) => {
		setSelectedTaskIds((current) => {
			const next = new Set(current);
			if (selected) next.add(item.id);
			else next.delete(item.id);
			return next;
		});
	}, []);

	const clearSelection = useCallback(() => setSelectedTaskIds(new Set()), []);

	function handleSelectRemoteProject(remoteProjectId: WorkProjectId) {
		if (remoteProjectId === selectedRemoteProjectId) return;
		setSelectedRemoteProjectId(remoteProjectId);
		setSelectedGroupId(null);
		clearSelection();
	}

	function handleSelectGroup(groupId: WorkItemGroupId | null) {
		setSelectedGroupId(groupId);
		clearSelection();
	}

	function openIntegrationSettings() {
		navigate(settingsPath(SettingsTab.Integrations));
	}

	return (
		<Stack gap="lg" w="100%" p="lg">
			<Title order={2}>{t("workTracking.title")}</Title>
			{status === ConnectionsStatus.Loading && <Loader size="sm" />}
			{status === ConnectionsStatus.Error && error && (
				<Alert color="red" title={t("common.error")}>
					<Stack gap="xs" align="flex-start">
						<Text size="sm">{translateError(t, error)}</Text>
						<Button size="xs" variant="light" onClick={reload}>
							{t("workTracking.retry")}
						</Button>
					</Stack>
				</Alert>
			)}
			{status === ConnectionsStatus.Ready && !connection && (
				<Stack gap={4}>
					<Text size="sm" c="dimmed">
						{t("workTracking.notConnected")}
					</Text>
					<Anchor size="sm" onClick={openIntegrationSettings}>
						{t("workTracking.goToSettings")}
					</Anchor>
				</Stack>
			)}
			<Select
				label={t("workTracking.link.localProject")}
				description={t("workTracking.link.localProjectHint")}
				placeholder={t("workTracking.link.localProjectPlaceholder")}
				data={projects.map((project) => ({
					value: project.tag,
					label: `${project.name} (${project.tag})`,
				}))}
				value={selectedProject?.tag ?? null}
				onChange={setSelectedProjectTag}
				disabled={loading}
				maw={400}
				searchable
				clearable
			/>
			{connection && (
				<Grid gap="lg">
					<Grid.Col span={{ base: 12, md: 4 }}>
						<RemoteProjectList
							list={projectList}
							selectedRemoteProjectId={selectedRemoteProjectId}
							onSelect={handleSelectRemoteProject}
							connection={connection}
							onReplaceKey={openIntegrationSettings}
						/>
					</Grid.Col>
					<Grid.Col span={{ base: 12, md: 4 }}>
						{selectedRemoteProjectId ? (
							<WorkItemGroupList
								list={groupList}
								selectedGroupId={selectedGroupId}
								onSelect={handleSelectGroup}
								onReplaceKey={openIntegrationSettings}
							/>
						) : (
							<ColumnHint title={t("workTracking.groups.title")}>
								{t("workTracking.groups.selectProject")}
							</ColumnHint>
						)}
					</Grid.Col>
					<Grid.Col span={{ base: 12, md: 4 }}>
						{selectedGroupId ? (
							<Stack gap="xs">
								<Group justify="flex-end">
									<Tooltip
										label={t("workTracking.bulkCreate.selectLocalProjectFirst")}
										disabled={canSelectTasks}
									>
										<Button
											variant="default"
											size="xs"
											leftSection={<SparkleIcon size={16} />}
											onClick={openBulk}
											disabled={!canSelectTasks || selectedTaskIds.size === 0}
										>
											{t("workTracking.bulkCreate.button", { count: selectedTaskIds.size })}
										</Button>
									</Tooltip>
								</Group>
								<WorkItemList
									list={itemList}
									connection={connection}
									onReplaceKey={openIntegrationSettings}
									selection={
										canSelectTasks
											? { selectedIds: selectedTaskIds, onToggle: toggleTaskSelected }
											: undefined
									}
								/>
							</Stack>
						) : (
							<ColumnHint title={t("workTracking.items.title")}>
								{t("workTracking.items.selectGroup")}
							</ColumnHint>
						)}
					</Grid.Col>
				</Grid>
			)}
			{selectedProject && (
				<BulkCreateFromTasksModal
					opened={bulkOpened}
					onClose={closeBulk}
					project={selectedProject}
					tasks={selectedTasks}
					onCreated={clearSelection}
				/>
			)}
		</Stack>
	);
}

function ColumnHint({ title, children }: { title: string; children: string }) {
	return (
		<Stack gap="xs">
			<Title order={3}>{title}</Title>
			<Text size="sm" c="dimmed">
				{children}
			</Text>
		</Stack>
	);
}
