import { Badge, Button, Group, Paper, Select, Stack, Text, Title } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { isOpenableProviderUrl } from "@/domain/work-tracking/providerUrl";
import type { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import type { RemoteProjectLinkActions } from "@/hooks/work-tracking/useRemoteProjectLinks";
import type { Project } from "@/models/Project";

type LocalProjectLinkPanelProps = {
	projects: Project[];
	loading: boolean;
	selectedProject: Project | null;
	onSelectProject: (projectTag: string | null) => void;
	connections: readonly ProviderConnection[];
	unlink: RemoteProjectLinkActions["unlink"];
	isPending: RemoteProjectLinkActions["isPending"];
};

export function LocalProjectLinkPanel({
	projects,
	loading,
	selectedProject,
	onSelectProject,
	connections,
	unlink,
	isPending,
}: LocalProjectLinkPanelProps) {
	const { t } = useTranslation();
	const projectOptions = projects.map((project) => ({
		value: project.tag,
		label: `${project.name} (${project.tag})`,
	}));

	return (
		<Paper withBorder p="md" radius="md">
			<Stack gap="sm">
				<Title order={3}>{t("workTracking.link.title")}</Title>
				<Select
					label={t("workTracking.link.localProject")}
					placeholder={t("workTracking.link.localProjectPlaceholder")}
					data={projectOptions}
					value={selectedProject?.tag ?? null}
					onChange={onSelectProject}
					disabled={loading}
					searchable
					clearable
				/>
				{!loading && projects.length === 0 && (
					<Text size="sm" c="dimmed">
						{t("workTracking.link.noLocalProjects")}
					</Text>
				)}
				{selectedProject && (
					<LinkedRemoteProjects
						project={selectedProject}
						connections={connections}
						unlink={unlink}
						isPending={isPending}
					/>
				)}
			</Stack>
		</Paper>
	);
}

type LinkedRemoteProjectsProps = {
	project: Project;
	connections: readonly ProviderConnection[];
	unlink: RemoteProjectLinkActions["unlink"];
	isPending: RemoteProjectLinkActions["isPending"];
};

function LinkedRemoteProjects({
	project,
	connections,
	unlink,
	isPending,
}: LinkedRemoteProjectsProps) {
	const { t } = useTranslation();
	const links = project.remoteProjectLinks.toArray();

	if (links.length === 0) {
		return (
			<Text size="sm" c="dimmed">
				{t("workTracking.link.noLinks")}
			</Text>
		);
	}

	return (
		<Stack component="ul" gap="xs" p={0} m={0} style={{ listStyle: "none" }}>
			{links.map((link) => (
				<LinkedRemoteProjectRow
					key={link.identityKey()}
					project={project}
					link={link}
					linkConnection={connections.find((candidate) => candidate.id === link.connectionId)}
					unlink={unlink}
					pending={isPending(project.tag, link.identityKey())}
				/>
			))}
		</Stack>
	);
}

type LinkedRemoteProjectRowProps = {
	project: Project;
	link: RemoteProjectLink;
	linkConnection: ProviderConnection | undefined;
	unlink: RemoteProjectLinkActions["unlink"];
	pending: boolean;
};

function LinkedRemoteProjectRow({
	project,
	link,
	linkConnection,
	unlink,
	pending,
}: LinkedRemoteProjectRowProps) {
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
				<Text size="sm" fw={500} truncate>
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
			<Group gap="xs" wrap="nowrap">
				{openableUrl && (
					<Button
						size="xs"
						variant="subtle"
						leftSection={<ArrowSquareOutIcon size={14} />}
						onClick={() => openUrl(openableUrl)}
						aria-label={t("workTracking.projects.openInProvider", { name: link.remoteProjectName })}
					>
						{t("workTracking.link.open")}
					</Button>
				)}
				<Button
					size="xs"
					variant="light"
					color="red"
					onClick={() => unlink(project, link)}
					disabled={pending}
					aria-busy={pending ? "true" : undefined}
					aria-label={t("workTracking.link.unlinkNamed", { name: link.remoteProjectName })}
				>
					{t("workTracking.link.unlink")}
				</Button>
			</Group>
		</Group>
	);
}
