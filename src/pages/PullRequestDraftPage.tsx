import {
	ActionIcon,
	Box,
	Button,
	Container,
	Group,
	Stack,
	Tabs,
	Text,
	TextInput,
	Title,
	Tooltip,
} from "@mantine/core";
import { useClipboard } from "@mantine/hooks";
import { ArrowLeftIcon, CheckIcon, CopyIcon } from "@phosphor-icons/react";
import MDEditor from "@uiw/react-md-editor";
import "@uiw/react-md-editor/markdown-editor.css";
import "./PullRequestDraftPage.css";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { useContexts } from "@/hooks/useContexts";
import { useProjects } from "@/hooks/useProjects";
import type { Context, Project, PullRequestDraft } from "@/models/Project";

const EMPTY_DRAFT: PullRequestDraft = { title: "", description: "" };

function buildMarkdown({ title, description }: PullRequestDraft): string {
	const trimmedTitle = title.trim();
	const trimmedDescription = description.trim();
	if (!trimmedTitle) return trimmedDescription;
	return `# ${trimmedTitle}\n\n${trimmedDescription}`;
}

type RepositoryDraftPanelProps = {
	repositoryId: string;
	branch: string;
	draft: PullRequestDraft;
	onChange: (repositoryId: string, draft: PullRequestDraft) => void;
};

function RepositoryDraftPanel({
	repositoryId,
	branch,
	draft,
	onChange,
}: RepositoryDraftPanelProps) {
	const { t } = useTranslation();
	const clipboard = useClipboard({ timeout: 1500 });

	const markdown = buildMarkdown(draft);

	return (
		<Tabs.Panel value={repositoryId} pt="md">
			<Stack gap="sm">
				<Text size="xs" c="dimmed" style={{ fontFamily: "monospace" }}>
					{branch}
				</Text>
				<TextInput
					label={t("contexts.prTitle")}
					placeholder={t("contexts.prTitlePlaceholder")}
					value={draft.title}
					onChange={(e) => onChange(repositoryId, { ...draft, title: e.currentTarget.value })}
				/>
				<Box>
					<Text size="sm" fw={500} mb={4}>
						{t("contexts.prDescription")}
					</Text>
					<Box data-color-mode="dark">
						<MDEditor
							className="pr-draft-editor"
							value={draft.description}
							onChange={(value) => onChange(repositoryId, { ...draft, description: value ?? "" })}
							height={560}
							textareaProps={{ placeholder: t("contexts.prDescriptionPlaceholder") }}
						/>
					</Box>
				</Box>
				<Group justify="flex-end">
					<Button
						variant="light"
						leftSection={clipboard.copied ? <CheckIcon size={16} /> : <CopyIcon size={16} />}
						color={clipboard.copied ? "green" : undefined}
						onClick={() => clipboard.copy(markdown)}
						disabled={!markdown.trim()}
					>
						{clipboard.copied ? t("common.copied") : t("contexts.copyMarkdown")}
					</Button>
				</Group>
			</Stack>
		</Tabs.Panel>
	);
}

type PullRequestDraftViewProps = {
	project: Project;
	context: Context;
};

function PullRequestDraftView({ project, context }: PullRequestDraftViewProps) {
	const { t } = useTranslation();
	const navigate = useNavigate();
	const { savePullRequestDrafts } = useContexts(project);
	const [drafts, setDrafts] = useState<Record<string, PullRequestDraft>>(context.pullRequestDrafts);

	const draftsRef = useRef(drafts);
	draftsRef.current = drafts;

	const saveRef = useRef(savePullRequestDrafts);
	saveRef.current = savePullRequestDrafts;

	const contextIdRef = useRef(context.id);
	contextIdRef.current = context.id;

	useEffect(() => {
		function autoSave() {
			saveRef.current(contextIdRef.current, draftsRef.current);
		}

		function handleVisibilityChange() {
			if (document.visibilityState === "hidden") {
				autoSave();
			}
		}

		window.addEventListener("blur", autoSave);
		document.addEventListener("visibilitychange", handleVisibilityChange);

		return () => {
			window.removeEventListener("blur", autoSave);
			document.removeEventListener("visibilitychange", handleVisibilityChange);
			// Persist on unmount (navigating away from the page).
			autoSave();
		};
	}, []);

	const repositories = context.branches
		.map((branch) => ({ branch, repository: project.findRepository(branch.repositoryId) }))
		.filter((entry) => entry.repository !== undefined);

	function handleDraftChange(repositoryId: string, draft: PullRequestDraft) {
		setDrafts((prev) => ({ ...prev, [repositoryId]: draft }));
	}

	function handleBack() {
		navigate("/pull-requests");
	}

	return (
		<Stack gap="md">
			<Group gap="sm">
				<Tooltip label={t("common.back")}>
					<ActionIcon variant="subtle" size="lg" onClick={handleBack} aria-label={t("common.back")}>
						<ArrowLeftIcon size={20} />
					</ActionIcon>
				</Tooltip>
				<Stack gap={0}>
					<Title order={3}>{t("contexts.draftPullRequestTitle")}</Title>
					<Text size="sm" c="dimmed">
						{t("contexts.draftPullRequestDescription", { name: context.name })}
					</Text>
				</Stack>
			</Group>
			{repositories.length === 0 ? (
				<Text size="sm" c="dimmed">
					{t("contexts.noRepositories")}
				</Text>
			) : (
				<Tabs defaultValue={repositories[0].branch.repositoryId}>
					<Tabs.List>
						{repositories.map(({ branch, repository }) => (
							<Tabs.Tab key={branch.repositoryId} value={branch.repositoryId}>
								{repository?.name}
							</Tabs.Tab>
						))}
					</Tabs.List>
					{repositories.map(({ branch }) => (
						<RepositoryDraftPanel
							key={branch.repositoryId}
							repositoryId={branch.repositoryId}
							branch={branch.branch}
							draft={drafts[branch.repositoryId] ?? EMPTY_DRAFT}
							onChange={handleDraftChange}
						/>
					))}
				</Tabs>
			)}
		</Stack>
	);
}

export function PullRequestDraftPage() {
	const { t } = useTranslation();
	const { projectTag, contextId } = useParams();
	const { projects, loading } = useProjects();

	if (loading) {
		return null;
	}

	const project = projects.find((p) => p.tag === projectTag);
	const context = project?.contexts.find((c) => c.id === contextId);

	return (
		<Container size="xl" py="lg">
			{project && context ? (
				<PullRequestDraftView project={project} context={context} />
			) : (
				<Text c="dimmed">{t("contexts.draftContextNotFound")}</Text>
			)}
		</Container>
	);
}
