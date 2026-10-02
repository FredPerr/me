import {
	Alert,
	Anchor,
	Button,
	Checkbox,
	Group,
	Loader,
	Modal,
	Stack,
	Tabs,
	Text,
	TextInput,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { ArrowSquareOutIcon, SparkleIcon, WarningIcon } from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import MDEditor from "@uiw/react-md-editor";
import "@uiw/react-md-editor/markdown-editor.css";
import "./CreatePullRequestModal.css";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { usePullRequestDescriptionDraft } from "@/hooks/usePullRequestDescriptionDraft";
import { usePushWorktree } from "@/hooks/usePushWorktree";
import type { CreatedPullRequest, RepositoryRef } from "@/models/git-provider/GitProvider";
import { getGitProvider } from "@/models/git-provider/GitProviderRegistry";
import type { Context, Project, PullRequestDraft, Repository } from "@/models/Project";

const github = getGitProvider("github");

/** Everything needed to open a pull request for one subrepository. */
type RepositoryTarget = {
	repository: Repository;
	/** Branch holding the context's changes (the PR head). */
	headBranch: string;
	/** Branch the PR merges into, taken from the base context. */
	baseBranch: string | null;
	/** owner/repo parsed from the remote, or null if not a GitHub remote. */
	ref: RepositoryRef | null;
};

type CreatePullRequestModalProps = {
	opened: boolean;
	onClose: () => void;
	project: Project;
	context: Context;
	allContexts: Context[];
	/** Persists the per-repository drafts (title + description). */
	onSaveDrafts: (
		contextId: string,
		drafts: Record<string, PullRequestDraft>,
	) => Promise<void> | void;
};

const EMPTY_DRAFT: PullRequestDraft = { title: "", description: "" };

async function resolveRef(project: Project, repository: Repository): Promise<RepositoryRef | null> {
	try {
		const resolvedPath = await repository.resolveAbsolutePath(project.path);
		const remoteUrl = await invoke<string | null>("get_git_remote_url", { path: resolvedPath });
		return remoteUrl ? github.parseRepositoryRef(remoteUrl) : null;
	} catch {
		return null;
	}
}

export function CreatePullRequestModal({
	opened,
	onClose,
	project,
	context,
	allContexts,
	onSaveDrafts,
}: CreatePullRequestModalProps) {
	const { t } = useTranslation();
	const { push } = usePushWorktree();
	const { stateFor, generate } = usePullRequestDescriptionDraft(project);

	const [targets, setTargets] = useState<RepositoryTarget[] | null>(null);
	const [drafts, setDrafts] = useState<Record<string, PullRequestDraft>>(context.pullRequestDrafts);
	const [creatingRepositoryId, setCreatingRepositoryId] = useState<string | null>(null);
	const [draftByRepository, setDraftByRepository] = useState<Record<string, boolean>>({});
	const [createdByRepository, setCreatedByRepository] = useState<
		Record<string, CreatedPullRequest>
	>({});

	const baseContext = useMemo(
		() => allContexts.find((candidate) => candidate.name === context.baseContextName),
		[allContexts, context.baseContextName],
	);

	// Resolve each subrepo's GitHub ref and base branch once the modal opens.
	useEffect(() => {
		if (!opened) return;
		let cancelled = false;

		async function resolveTargets() {
			const resolved = await Promise.all(
				context.branches.map(async (branch) => {
					const repository = project.findRepository(branch.repositoryId);
					if (!repository) return null;
					const ref = await resolveRef(project, repository);
					return {
						repository,
						headBranch: branch.branch,
						baseBranch: baseContext?.getBranchForRepository(branch.repositoryId) ?? null,
						ref,
					} satisfies RepositoryTarget;
				}),
			);
			if (!cancelled) {
				setTargets(resolved.filter((target): target is RepositoryTarget => target !== null));
			}
		}

		resolveTargets();
		return () => {
			cancelled = true;
		};
	}, [opened, context.branches, project, baseContext]);

	// Fold a freshly generated description into the editable draft.
	useEffect(() => {
		if (!targets) return;
		for (const target of targets) {
			const generated = stateFor(target.repository.id);
			if (generated.status === "ready" && generated.description) {
				setDrafts((current) => {
					const existing = current[target.repository.id] ?? EMPTY_DRAFT;
					if (existing.description === generated.description) return current;
					return {
						...current,
						[target.repository.id]: { ...existing, description: generated.description },
					};
				});
			}
		}
	}, [targets, stateFor]);

	function draftFor(repositoryId: string): PullRequestDraft {
		return drafts[repositoryId] ?? EMPTY_DRAFT;
	}

	function updateDraft(repositoryId: string, next: PullRequestDraft) {
		setDrafts((current) => ({ ...current, [repositoryId]: next }));
	}

	function handleClose() {
		onSaveDrafts(context.id, drafts);
		onClose();
	}

	async function handleCreate(target: RepositoryTarget) {
		if (!target.ref || !target.baseBranch) return;
		const draft = draftFor(target.repository.id);
		if (!draft.title.trim()) {
			notifications.show({
				color: "red",
				title: t("contexts.createPr.missingTitle"),
				message: t("contexts.createPr.missingTitleMessage"),
			});
			return;
		}

		setCreatingRepositoryId(target.repository.id);
		try {
			const worktreePath = context.getWorktreePath(project.path, target.repository);
			const pushResult = await push(worktreePath);
			if (!pushResult.pushed) {
				notifications.show({
					color: "red",
					title: t("contexts.createPr.pushFailed"),
					message: pushResult.message ?? t("contexts.createPr.pushFailed"),
				});
				return;
			}

			const created = await github.createPullRequest(target.ref, {
				title: draft.title.trim(),
				body: draft.description,
				head: target.headBranch,
				base: target.baseBranch,
				draft: draftByRepository[target.repository.id] ?? false,
			});

			setCreatedByRepository((current) => ({ ...current, [target.repository.id]: created }));
			await onSaveDrafts(context.id, drafts);
			notifications.show({
				color: "green",
				title: t("contexts.createPr.createdTitle"),
				message: t("contexts.createPr.createdMessage", { number: created.number }),
			});
		} catch (error) {
			notifications.show({
				color: "red",
				title: t("contexts.createPr.createFailed"),
				message: String(error),
			});
		} finally {
			setCreatingRepositoryId(null);
		}
	}

	function renderPanel(target: RepositoryTarget) {
		const repositoryId = target.repository.id;
		const draft = draftFor(repositoryId);
		const generation = stateFor(repositoryId);
		const created = createdByRepository[repositoryId];
		const canCreate = Boolean(target.ref) && Boolean(target.baseBranch);

		return (
			<Tabs.Panel key={repositoryId} value={repositoryId} pt="md">
				<Stack gap="sm">
					{!target.ref && (
						<Alert color="orange" icon={<WarningIcon size={16} />}>
							{t("contexts.createPr.notGitHub")}
						</Alert>
					)}
					{target.ref && !target.baseBranch && (
						<Alert color="orange" icon={<WarningIcon size={16} />}>
							{t("contexts.createPr.noBaseBranch")}
						</Alert>
					)}
					{canCreate && (
						<Text size="xs" c="dimmed" style={{ fontFamily: "monospace" }}>
							{target.headBranch} → {target.baseBranch}
						</Text>
					)}

					<TextInput
						label={t("contexts.prTitle")}
						placeholder={t("contexts.prTitlePlaceholder")}
						value={draft.title}
						disabled={!canCreate}
						onChange={(event) =>
							updateDraft(repositoryId, { ...draft, title: event.currentTarget.value })
						}
					/>

					<Stack gap={4}>
						<Group justify="space-between" align="flex-end">
							<Text size="sm" fw={500}>
								{t("contexts.prDescription")}
							</Text>
							<Button
								size="xs"
								variant="light"
								leftSection={
									generation.status === "generating" ? (
										<Loader size={12} />
									) : (
										<SparkleIcon size={14} />
									)
								}
								disabled={!canCreate || generation.status === "generating"}
								onClick={() =>
									target.baseBranch &&
									generate({
										context,
										repository: target.repository,
										baseBranch: target.baseBranch,
									})
								}
							>
								{generation.status === "generating"
									? t("contexts.createPr.generating")
									: t("contexts.createPr.generate")}
							</Button>
						</Group>
						{generation.status === "error" && generation.error && (
							<Alert color="red" icon={<WarningIcon size={16} />}>
								{generation.error}
							</Alert>
						)}
						<div data-color-mode="dark">
							<MDEditor
								className="pr-draft-editor"
								value={draft.description}
								height={420}
								onChange={(value) =>
									updateDraft(repositoryId, { ...draft, description: value ?? "" })
								}
								textareaProps={{ placeholder: t("contexts.prDescriptionPlaceholder") }}
							/>
						</div>
					</Stack>

					<Group justify="space-between">
						<Checkbox
							label={t("contexts.createPr.createAsDraft")}
							checked={draftByRepository[repositoryId] ?? false}
							disabled={!canCreate || Boolean(created)}
							onChange={(event) =>
								setDraftByRepository((current) => ({
									...current,
									[repositoryId]: event.currentTarget.checked,
								}))
							}
						/>
						{created ? (
							<Anchor href={created.htmlUrl} target="_blank" rel="noreferrer">
								<Group gap={6}>
									<ArrowSquareOutIcon size={16} />
									{t("contexts.createPr.viewPr", { number: created.number })}
								</Group>
							</Anchor>
						) : (
							<Button
								disabled={!canCreate}
								loading={creatingRepositoryId === repositoryId}
								onClick={() => handleCreate(target)}
							>
								{draftByRepository[repositoryId]
									? t("contexts.createPr.createDraft")
									: t("contexts.createPr.create")}
							</Button>
						)}
					</Group>
				</Stack>
			</Tabs.Panel>
		);
	}

	return (
		<Modal opened={opened} onClose={handleClose} title={t("contexts.createPr.title")} size="xl">
			{targets === null ? (
				<Group justify="center" py="xl">
					<Loader />
				</Group>
			) : targets.length === 0 ? (
				<Text size="sm" c="dimmed">
					{t("contexts.noRepositories")}
				</Text>
			) : (
				<Tabs defaultValue={targets[0].repository.id}>
					<Tabs.List>
						{targets.map((target) => (
							<Tabs.Tab key={target.repository.id} value={target.repository.id}>
								{target.repository.name}
							</Tabs.Tab>
						))}
					</Tabs.List>
					{targets.map(renderPanel)}
				</Tabs>
			)}
		</Modal>
	);
}
