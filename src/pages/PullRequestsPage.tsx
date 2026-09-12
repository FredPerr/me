import { Badge, Card, Container, Group, Stack, Tabs, Text, Title } from "@mantine/core";
import { GitPullRequestIcon, PlugsIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { useProjects } from "@/hooks/useProjects";
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

function ActiveSection() {
	const { t } = useTranslation();

	return (
		<Stack align="center" gap="sm" py="xl">
			<PlugsIcon size={40} color="var(--mantine-color-dark-2)" />
			<Text fw={600}>{t("pullRequests.notConnectedTitle")}</Text>
			<Text size="sm" c="dimmed" ta="center" maw={420}>
				{t("pullRequests.notConnectedDescription")}
			</Text>
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
						<ActiveSection />
					</Tabs.Panel>
					<Tabs.Panel value="drafts" pt="md">
						<DraftsSection projects={projects} />
					</Tabs.Panel>
				</Tabs>
			</Stack>
		</Container>
	);
}
