import { Alert, Anchor, Badge, Card, Group, Loader, Stack, Tabs, Text, Title } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { ArrowSquareOutIcon, GitPullRequestIcon, PlugsIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import type {
	ContextView,
	HostedPullRequestView,
	HostKindView,
	ProjectView,
	PullRequestDraftView,
	RepositoryPullRequestsView,
} from "@/bindings";
import { CreatePullRequestModal } from "@/components/contexts/CreatePullRequestModal";
import { ProviderBadge } from "@/components/shared/ProviderBadge";
import { PullRequestsNotices } from "@/components/shared/PullRequestsNotices";
import { useProjectsPullRequests } from "@/hooks/gitHosting/useProjectPullRequests";
import { useProjects } from "@/hooks/workspace/useProjects";
import { describeError } from "@/infra/commandError";
import {
	isEveryHostNotConnected,
	listedPullRequests,
	pullRequestsNotices,
} from "@/models/git-hosting/repositoryPullRequests";
import { findEffectiveRepository } from "@/models/projectView";
import { SettingsTab, settingsPath } from "./settingsTabs";

type DraftEntry = {
	project: ProjectView;
	context: ContextView;
	repositoryId: string;
	repositoryName: string;
	draft: PullRequestDraftView;
};

function isDraftFilled(draft: PullRequestDraftView): boolean {
	return draft.title.trim().length > 0 || draft.description.trim().length > 0;
}

function collectDraftEntries(projects: ProjectView[]): DraftEntry[] {
	const entries: DraftEntry[] = [];
	for (const project of projects) {
		for (const context of project.contexts) {
			for (const { repositoryId, pullRequestDraft } of context.branches) {
				if (!pullRequestDraft || !isDraftFilled(pullRequestDraft)) continue;
				const repository = findEffectiveRepository(project, repositoryId);
				if (!repository) continue;
				entries.push({
					project,
					context,
					repositoryId,
					repositoryName: repository.name,
					draft: pullRequestDraft,
				});
			}
		}
	}
	return entries;
}

function DraftEntryCard({ entry }: { entry: DraftEntry }) {
	const { t } = useTranslation();
	const [opened, { open, close }] = useDisclosure(false);

	return (
		<>
			<Card withBorder padding="sm" onClick={open} style={{ cursor: "pointer" }}>
				<Stack gap={4}>
					<Group justify="space-between" gap="xs">
						<Text fw={600} size="sm">
							{entry.draft.title.trim() || t("pullRequests.untitledDraft")}
						</Text>
						<Badge variant="light" size="sm">
							{entry.repositoryName}
						</Badge>
					</Group>
					<Text size="xs" c="dimmed">
						{entry.project.name} — {entry.context.name}
					</Text>
				</Stack>
			</Card>

			<CreatePullRequestModal
				opened={opened}
				onClose={close}
				project={entry.project}
				context={entry.context}
			/>
		</>
	);
}

function DraftsSection({ projects }: { projects: ProjectView[] }) {
	const { t } = useTranslation();
	const entries = collectDraftEntries(projects);

	if (entries.length === 0) {
		return (
			<Text size="sm" c="dimmed">
				{t("pullRequests.noDrafts")}
			</Text>
		);
	}

	return (
		<Stack gap="sm">
			{entries.map((entry) => (
				<DraftEntryCard key={`${entry.context.id}-${entry.repositoryId}`} entry={entry} />
			))}
		</Stack>
	);
}

function NotConnected() {
	const { t } = useTranslation();
	const navigate = useNavigate();

	return (
		<Stack align="center" gap="sm" py="xl">
			<PlugsIcon size={40} color="var(--mantine-color-dark-2)" />
			<Text fw={600}>{t("pullRequests.notConnectedTitle")}</Text>
			<Text size="sm" c="dimmed" ta="center" maw={420}>
				{t("pullRequests.notConnectedDescription")}
			</Text>
			<Anchor size="sm" onClick={() => navigate(settingsPath(SettingsTab.Integrations))}>
				{t("pullRequests.goToSettings")}
			</Anchor>
		</Stack>
	);
}

function PullRequestCard({
	pullRequest,
	host,
}: {
	pullRequest: HostedPullRequestView;
	host: HostKindView | null;
}) {
	const { t } = useTranslation();
	return (
		<Card
			withBorder
			padding="sm"
			onClick={() => openUrl(pullRequest.url)}
			style={{ cursor: "pointer" }}
		>
			<Group justify="space-between" gap="xs" wrap="nowrap">
				<Stack gap={2} style={{ minWidth: 0 }}>
					<Group gap="xs" wrap="nowrap">
						<Text fw={600} size="sm" truncate>
							{pullRequest.title}
						</Text>
						{host && <ProviderBadge host={host} />}
						{pullRequest.draft && (
							<Badge variant="light" color="gray" size="sm">
								{t("pullRequests.draftBadge")}
							</Badge>
						)}
					</Group>
					<Text size="xs" c="dimmed">
						{t("pullRequests.prMeta", {
							number: pullRequest.number,
							author: pullRequest.author ?? t("pullRequests.unknownAuthor"),
							branch: pullRequest.headBranch,
						})}
					</Text>
				</Stack>
				<ArrowSquareOutIcon size={16} />
			</Group>
		</Card>
	);
}

type ProjectRepositoryPullRequests = {
	project: ProjectView;
	repository: RepositoryPullRequestsView;
};

function RepositoryGroup({ group }: { group: ProjectRepositoryPullRequests }) {
	return (
		<Stack gap="xs">
			<Group gap="xs">
				<Text fw={600} size="sm">
					{group.project.name}
				</Text>
				<Badge variant="light" size="sm">
					{group.repository.repositoryName}
				</Badge>
			</Group>
			{listedPullRequests(group.repository).map((pullRequest) => (
				<PullRequestCard
					key={pullRequest.number}
					pullRequest={pullRequest}
					host={group.repository.host}
				/>
			))}
		</Stack>
	);
}

function ActiveSection({ projects }: { projects: ProjectView[] }) {
	const { t } = useTranslation();
	const projectQueries = useProjectsPullRequests(projects.map((project) => project.tag));
	if (projectQueries.some((query) => query.isPending)) {
		return (
			<Group justify="center" py="xl">
				<Loader size="sm" />
			</Group>
		);
	}
	const groups: ProjectRepositoryPullRequests[] = projects.flatMap((project, index) =>
		(projectQueries[index]?.data ?? []).map((repository) => ({ project, repository })),
	);
	const queryErrors = projectQueries.flatMap((query) => (query.error ? [query.error] : []));
	if (
		queryErrors.length === 0 &&
		isEveryHostNotConnected(groups.map((group) => group.repository))
	) {
		return <NotConnected />;
	}
	const notices = pullRequestsNotices(
		groups.map(({ project, repository }) => ({
			label: `${project.name} / ${repository.repositoryName}`,
			repository,
		})),
	);
	const groupsWithPullRequests = groups.filter(
		(group) => listedPullRequests(group.repository).length > 0,
	);
	return (
		<Stack gap="lg">
			{queryErrors.length > 0 && (
				<Alert color="red" title={t("common.error")}>
					<Stack gap={4}>
						<Text size="sm">{t("pullRequests.loadError")}</Text>
						{queryErrors.map((error) => (
							<Text
								key={describeError(error)}
								size="xs"
								c="dimmed"
								style={{ wordBreak: "break-word" }}
							>
								{describeError(error)}
							</Text>
						))}
					</Stack>
				</Alert>
			)}
			<PullRequestsNotices notices={notices} />
			{groupsWithPullRequests.length === 0 ? (
				<Text size="sm" c="dimmed" ta="center" py="xl">
					{t("pullRequests.noOpen")}
				</Text>
			) : (
				groupsWithPullRequests.map((group) => (
					<RepositoryGroup
						key={`${group.project.tag}-${group.repository.repositoryId}`}
						group={group}
					/>
				))
			)}
		</Stack>
	);
}

export function PullRequestsPage() {
	const { t } = useTranslation();
	const { projects, loading } = useProjects();

	if (loading) {
		return null;
	}

	return (
		<Stack gap="lg" w="100%" p="lg">
			<Title order={2}>{t("pullRequests.title")}</Title>
			<Tabs defaultValue="active">
				<Tabs.List grow>
					<Tabs.Tab value="active" leftSection={<GitPullRequestIcon size={16} />}>
						{t("pullRequests.activeTab")}
					</Tabs.Tab>
					<Tabs.Tab value="drafts" leftSection={<GitPullRequestIcon size={16} />}>
						{t("pullRequests.draftsTab")}
					</Tabs.Tab>
				</Tabs.List>
				<Tabs.Panel value="active" pt="md">
					<ActiveSection projects={projects} />
				</Tabs.Panel>
				<Tabs.Panel value="drafts" pt="md">
					<DraftsSection projects={projects} />
				</Tabs.Panel>
			</Tabs>
		</Stack>
	);
}
