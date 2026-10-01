import { Container, Grid, Stack, Text, Title } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ConnectionPanel } from "@/components/work-tracking/ConnectionPanel";
import { LocalProjectLinkPanel } from "@/components/work-tracking/LocalProjectLinkPanel";
import { RemoteProjectList } from "@/components/work-tracking/RemoteProjectList";
import { ReplaceKeyModal } from "@/components/work-tracking/ReplaceKeyModal";
import { WorkItemGroupList } from "@/components/work-tracking/WorkItemGroupList";
import { WorkItemList } from "@/components/work-tracking/WorkItemList";
import type { WorkItemGroupId, WorkProjectId } from "@/domain/work-tracking/identifiers";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { DEFAULT_PROVIDER_KIND } from "@/domain/work-tracking/ProviderKind";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import type { WorkItemGroup } from "@/domain/work-tracking/WorkItemGroup";
import type { WorkProject } from "@/domain/work-tracking/WorkProject";
import { usePagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";
import { useRemoteProjectLinks } from "@/hooks/work-tracking/useRemoteProjectLinks";
import { useWorkTrackingConnections } from "@/hooks/work-tracking/useWorkTrackingConnections";
import { workTrackingGateway } from "@/infra/work-tracking/workTracking";

export function TasksPage() {
	const { t } = useTranslation();
	const { connections, status, error, revision, save, remove, reload } =
		useWorkTrackingConnections();
	const { projects, loading, link, unlink, isPending } = useRemoteProjectLinks();
	const [selectedProjectTag, setSelectedProjectTag] = useState<string | null>(null);
	const [selectedRemoteProjectId, setSelectedRemoteProjectId] = useState<WorkProjectId | null>(
		null,
	);
	const [selectedGroupId, setSelectedGroupId] = useState<WorkItemGroupId | null>(null);
	const [replaceKeyOpened, setReplaceKeyOpened] = useState(false);

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

	function handleSelectRemoteProject(remoteProjectId: WorkProjectId) {
		if (remoteProjectId === selectedRemoteProjectId) return;
		setSelectedRemoteProjectId(remoteProjectId);
		setSelectedGroupId(null);
	}

	async function handleRemove(removedConnection: ProviderConnection) {
		await remove(removedConnection.id);
		setSelectedRemoteProjectId(null);
		setSelectedGroupId(null);
		setReplaceKeyOpened(false);
	}

	function openReplaceKey() {
		setReplaceKeyOpened(true);
	}

	return (
		<Container size="xl" py="lg">
			<Stack gap="lg">
				<Title order={2}>{t("workTracking.title")}</Title>
				<ConnectionPanel
					connection={connection}
					status={status}
					error={error}
					save={save}
					remove={handleRemove}
					onReplaceKey={openReplaceKey}
					onRetry={reload}
				/>
				<LocalProjectLinkPanel
					projects={projects}
					loading={loading}
					selectedProject={selectedProject}
					onSelectProject={setSelectedProjectTag}
					connections={connections}
					unlink={unlink}
					isPending={isPending}
				/>
				{connection && (
					<Grid gap="lg">
						<Grid.Col span={{ base: 12, md: 4 }}>
							<RemoteProjectList
								list={projectList}
								selectedRemoteProjectId={selectedRemoteProjectId}
								onSelect={handleSelectRemoteProject}
								selectedProject={selectedProject}
								connection={connection}
								link={link}
								unlink={unlink}
								isPending={isPending}
								onReplaceKey={openReplaceKey}
							/>
						</Grid.Col>
						<Grid.Col span={{ base: 12, md: 4 }}>
							{selectedRemoteProjectId ? (
								<WorkItemGroupList
									list={groupList}
									selectedGroupId={selectedGroupId}
									onSelect={setSelectedGroupId}
									onReplaceKey={openReplaceKey}
								/>
							) : (
								<ColumnHint title={t("workTracking.groups.title")}>
									{t("workTracking.groups.selectProject")}
								</ColumnHint>
							)}
						</Grid.Col>
						<Grid.Col span={{ base: 12, md: 4 }}>
							{selectedGroupId ? (
								<WorkItemList
									list={itemList}
									connection={connection}
									onReplaceKey={openReplaceKey}
								/>
							) : (
								<ColumnHint title={t("workTracking.items.title")}>
									{t("workTracking.items.selectGroup")}
								</ColumnHint>
							)}
						</Grid.Col>
					</Grid>
				)}
			</Stack>
			<ReplaceKeyModal
				opened={replaceKeyOpened}
				onClose={() => setReplaceKeyOpened(false)}
				connection={connection}
				save={save}
			/>
		</Container>
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
