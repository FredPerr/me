import {
	Alert,
	Anchor,
	Badge,
	Card,
	Container,
	Group,
	Loader,
	Stack,
	Tabs,
	Text,
	Title,
} from "@mantine/core";
import { ArrowSquareOutIcon, GitPullRequestIcon, PlugsIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import {
	type RepositoryPullRequests,
	useProjectPullRequests,
} from "@/hooks/useProjectPullRequests";
import { useProjects } from "@/hooks/useProjects";
import type { ProviderPullRequest } from "@/models/git-provider/GitProvider";
import type { Context, Project, PullRequestDraft } from "@/models/Project";

type DraftEntry = {
	project: Project;
	context: Context;
	repositoryId: string;
	repositoryName: string;
	draft: PullRequestDraft;
};

function isDraftFilled(draft: PullRequestDraft): boolean {
	return draft.title.trim().length > 0 || draft.description.trim().length > 0;
}

function collectDraftEntries(projects: Project[]): DraftEntry[] {
	const entries: DraftEntry[] = [];
	for (const project of projects) {
		for (const context of project.contexts) {
			for (const [repositoryId, draft] of Object.entries(context.pullRequestDrafts)) {
				if (!isDraftFilled(draft)) continue;
				const repository = project.findRepository(repositoryId);
				if (!repository) continue;
				entries.push({
					project,
					context,
					repositoryId,
					repositoryName: repository.name,
					draft,
				});
			}
		}
	}
	return entries;
}

function DraftEntryCard({ entry }: { entry: DraftEntry }) {
	const { t } = useTranslation();
	const navigate = useNavigate();

	function handleOpen() {
		navigate(`/projects/${entry.project.tag}/contexts/${entry.context.id}/pull-requests`);
	}

	return (
		<Card withBorder padding="sm" onClick={handleOpen} style={{ cursor: "pointer" }}>
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
	);
}

function DraftsSection({ projects }: { projects: Project[] }) {
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
			<Anchor size="sm" onClick={() => navigate("/settings")}>
				{t("pullRequests.goToSettings")}
			</Anchor>
		</Stack>
	);
}

function PullRequestCard({ pullRequest }: { pullRequest: ProviderPullRequest }) {
	const { t } = useTranslation();

	return (
		<Card
			withBorder
			padding="sm"
			onClick={() => openUrl(pullRequest.htmlUrl)}
			style={{ cursor: "pointer" }}
		>
			<Group justify="space-between" gap="xs" wrap="nowrap">
				<Stack gap={2} style={{ minWidth: 0 }}>
					<Group gap="xs" wrap="nowrap">
						<Text fw={600} size="sm" truncate>
							{pullRequest.title}
						</Text>
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

function RepositoryGroup({ group }: { group: RepositoryPullRequests }) {
	const { t } = useTranslation();

	return (
		<Stack gap="xs">
			<Group gap="xs">
				<Text fw={600} size="sm">
					{group.project.name}
				</Text>
				<Badge variant="light" size="sm">
					{group.repository.name}
				</Badge>
			</Group>
			{group.pullRequests.length === 0 ? (
				<Text size="xs" c="dimmed">
					{t("pullRequests.noOpenForRepo")}
				</Text>
			) : (
				group.pullRequests.map((pullRequest) => (
					<PullRequestCard key={pullRequest.number} pullRequest={pullRequest} />
				))
			)}
		</Stack>
	);
}

function ActiveSection({ projects }: { projects: Project[] }) {
	const { t } = useTranslation();
	const { connected, loading, groups, error } = useProjectPullRequests(projects);

	if (loading) {
		return (
			<Group justify="center" py="xl">
				<Loader size="sm" />
			</Group>
		);
	}

	if (!connected) {
		return <NotConnected />;
	}

	if (error) {
		return (
			<Alert color="red" title={t("common.error")}>
				{t("pullRequests.loadError")}
			</Alert>
		);
	}

	const groupsWithPullRequests = groups.filter((group) => group.pullRequests.length > 0);

	if (groupsWithPullRequests.length === 0) {
		return (
			<Text size="sm" c="dimmed" ta="center" py="xl">
				{t("pullRequests.noOpen")}
			</Text>
		);
	}

	return (
		<Stack gap="lg">
			{groupsWithPullRequests.map((group) => (
				<RepositoryGroup key={`${group.project.tag}-${group.repository.id}`} group={group} />
			))}
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
		<Container size="md" py="lg">
			<Stack gap="lg">
				<Title order={2}>{t("pullRequests.title")}</Title>
				<Tabs defaultValue="active">
					<Tabs.List>
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
		</Container>
	);
}
