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
import MDEditor from "@uiw/react-md-editor";
import "@uiw/react-md-editor/markdown-editor.css";
import "./CreatePullRequestModal.css";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
	ContextView,
	CreatedPullRequestView,
	ProjectView,
	PullRequestDraftView,
	RepositoryView,
} from "@/bindings";
import { useContextPullRequests } from "@/hooks/gitHosting/useContextPullRequests";
import { useCreatePullRequest } from "@/hooks/gitHosting/useGitHostingMutations";
import { usePullRequestDescriptionDraft } from "@/hooks/usePullRequestDescriptionDraft";
import { useSavePullRequestDraft } from "@/hooks/workspace/useContextMutations";
import { describeError } from "@/infra/commandError";
import type { BranchHostProblem } from "@/models/git-hosting/branchPullRequests";
import { GIT_HOSTS } from "@/models/git-hosting/gitHosts";
import {
	findBaseContext,
	findContextBranch,
	findDefaultBranch,
	findEffectiveRepository,
} from "@/models/projectView";

/** Everything needed to open a pull request for one subrepository. */
type RepositoryTarget = {
	repository: RepositoryView;
	/** The context's checkout of this repository, read to generate a description. */
	worktreePath: string;
	/** Branch holding the context's changes (the PR head). */
	headBranch: string;
	/** Branch the PR merges into: the base context's branch, else the default branch. */
	baseBranch: string | null;
};

type CreatePullRequestModalProps = {
	opened: boolean;
	onClose: () => void;
	project: ProjectView;
	context: ContextView;
};

const EMPTY_DRAFT: PullRequestDraftView = { title: "", description: "" };

function savedDrafts(context: ContextView): Record<string, PullRequestDraftView> {
	return Object.fromEntries(
		context.branches.flatMap((branch) =>
			branch.pullRequestDraft ? [[branch.repositoryId, branch.pullRequestDraft]] : [],
		),
	);
}

function isSameDraft(left: PullRequestDraftView, right: PullRequestDraftView): boolean {
	return left.title === right.title && left.description === right.description;
}

export function CreatePullRequestModal({
	opened,
	onClose,
	project,
	context,
}: CreatePullRequestModalProps) {
	const { t } = useTranslation();
	const { stateFor, generate } = usePullRequestDescriptionDraft();
	const { mutateAsync: savePullRequestDraft } = useSavePullRequestDraft(project.tag);
	const { mutateAsync: createPullRequest } = useCreatePullRequest(project.tag);
	const { hostProblem } = useContextPullRequests(project.tag);

	const [drafts, setDrafts] = useState<Record<string, PullRequestDraftView>>(() =>
		savedDrafts(context),
	);
	const [creatingRepositoryId, setCreatingRepositoryId] = useState<string | null>(null);
	const [draftByRepository, setDraftByRepository] = useState<Record<string, boolean>>({});
	const [createdByRepository, setCreatedByRepository] = useState<
		Record<string, CreatedPullRequestView>
	>({});

	const baseContext = useMemo(() => findBaseContext(project, context), [project, context]);

	const targets = useMemo(
		() =>
			context.branches.flatMap((branch): RepositoryTarget[] => {
				const repository = findEffectiveRepository(project, branch.repositoryId);
				if (!repository || !branch.worktreePath) return [];
				// Prefer the base context's branch; fall back to the repository's
				// default branch so a context whose base has no recorded branch for
				// this repo (e.g. a mono-repo default saved with empty branches)
				// still has something to open the pull request against.
				const baseBranch =
					(baseContext && findContextBranch(baseContext, branch.repositoryId)?.branch) ??
					findDefaultBranch(project, branch.repositoryId) ??
					null;
				return [
					{
						repository,
						worktreePath: branch.worktreePath,
						headBranch: branch.branch,
						baseBranch,
					},
				];
			}),
		[context.branches, project, baseContext],
	);

	// Fold a freshly generated description into the editable draft.
	useEffect(() => {
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

	function draftFor(repositoryId: string): PullRequestDraftView {
		return drafts[repositoryId] ?? EMPTY_DRAFT;
	}

	function updateDraft(repositoryId: string, next: PullRequestDraftView) {
		setDrafts((current) => ({ ...current, [repositoryId]: next }));
	}

	/** Saves the drafts that differ from the saved ones, one repository at a time. */
	async function saveChangedDrafts() {
		const saved = savedDrafts(context);
		for (const [repositoryId, draft] of Object.entries(drafts)) {
			if (isSameDraft(draft, saved[repositoryId] ?? EMPTY_DRAFT)) continue;
			await savePullRequestDraft({ contextId: context.id, repositoryId, ...draft });
		}
	}

	function handleClose() {
		saveChangedDrafts().catch(() => undefined);
		onClose();
	}

	function describeHostProblem(problem: BranchHostProblem): string {
		switch (problem.kind) {
			case "noRemote":
				return t("contexts.createPr.noRemote");
			case "unsupportedHost":
				return t("contexts.createPr.unsupportedRemote");
			case "notConnected":
				return t("contexts.createPr.notConnected", {
					host: problem.host
						? t(GIT_HOSTS[problem.host].displayNameKey)
						: t("contexts.createPr.unknownGitHost"),
				});
		}
	}

	async function handleCreate(target: RepositoryTarget) {
		if (!target.baseBranch) return;
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
			const created = await createPullRequest({
				repositoryId: target.repository.id,
				input: {
					title: draft.title.trim(),
					body: draft.description,
					head: target.headBranch,
					base: target.baseBranch,
					draft: draftByRepository[target.repository.id] ?? false,
				},
			});

			setCreatedByRepository((current) => ({ ...current, [target.repository.id]: created }));
			await saveChangedDrafts();
			notifications.show({
				color: "green",
				title: t("contexts.createPr.createdTitle"),
				message: t("contexts.createPr.createdMessage", { number: created.number }),
			});
		} catch (error) {
			notifications.show({
				color: "red",
				title: t("contexts.createPr.createFailed"),
				message: describeError(error),
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
		const problem = hostProblem(repositoryId, target.headBranch);
		const sameBranch = target.baseBranch === target.headBranch;
		const canCreate = !problem && Boolean(target.baseBranch) && !sameBranch;

		return (
			<Tabs.Panel key={repositoryId} value={repositoryId} pt="md">
				<Stack gap="sm">
					{problem && (
						<Alert color="orange" icon={<WarningIcon size={16} />}>
							{describeHostProblem(problem)}
						</Alert>
					)}
					{!problem && !target.baseBranch && (
						<Alert color="orange" icon={<WarningIcon size={16} />}>
							{t("contexts.createPr.noBaseBranch")}
						</Alert>
					)}
					{!problem && target.baseBranch && sameBranch && (
						<Alert color="orange" icon={<WarningIcon size={16} />}>
							{t("contexts.createPr.sameBranch", { branch: target.headBranch })}
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
										contextName: context.name,
										repository: target.repository,
										worktreePath: target.worktreePath,
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
							<Anchor href={created.url} target="_blank" rel="noreferrer">
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
			{targets.length === 0 ? (
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
