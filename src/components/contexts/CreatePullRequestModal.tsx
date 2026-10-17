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
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	type AiAssistErrorKind,
	type ContextView,
	type CreatedPullRequestView,
	commands,
	type ProjectView,
	type PullRequestDraftView,
	type RepositoryView,
} from "@/bindings";
import { useContextPullRequests } from "@/hooks/gitHosting/useContextPullRequests";
import { useCreatePullRequest } from "@/hooks/gitHosting/useGitHostingMutations";
import { useSavePullRequestDraft } from "@/hooks/workspace/useContextMutations";
import { describeError, isCommandError } from "@/infra/commandError";
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
	/** Branch holding the context's changes (the PR head). */
	headBranch: string;
	/** Branch the PR merges into: the base context's branch, else the default branch. */
	baseBranch: string | null;
};

/** Per-repository generation state, keyed by repository id. */
type GenerationState = {
	generatingRepositoryId: string | null;
	errorByRepository: Record<string, string>;
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

/** A copy of `record` without `key`. */
function withoutKey<T>(record: Record<string, T>, key: string): Record<string, T> {
	const { [key]: _removed, ...rest } = record;
	return rest;
}

export function CreatePullRequestModal({
	opened,
	onClose,
	project,
	context,
}: CreatePullRequestModalProps) {
	const { t } = useTranslation();
	const { mutateAsync: savePullRequestDraft } = useSavePullRequestDraft(project.tag);
	const { mutateAsync: createPullRequest } = useCreatePullRequest(project.tag);
	const { hostProblem } = useContextPullRequests(project.tag);

	const [drafts, setDrafts] = useState<Record<string, PullRequestDraftView>>(() =>
		savedDrafts(context),
	);
	const [generation, setGeneration] = useState<GenerationState>({
		generatingRepositoryId: null,
		errorByRepository: {},
	});
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
				return [{ repository, headBranch: branch.branch, baseBranch }];
			}),
		[context.branches, project, baseContext],
	);

	/** The i18n key for a generation failure, mirrored from the backend kinds. */
	function generationErrorKey(kind: AiAssistErrorKind): string {
		switch (kind) {
			case "emptyDiff":
				return "contexts.createPr.errorEmptyDiff";
			case "emptyAnswer":
				return "contexts.createPr.errorEmptyAnswer";
			case "timedOut":
				return "contexts.createPr.errorTimedOut";
			default:
				return "contexts.createPr.errorGeneration";
		}
	}

	/**
	 * Writes the description for one repository with a read-only Rust run. The
	 * returned description always fills the field; the returned title fills the
	 * title only when the user has not typed one, so their wording is kept.
	 */
	async function handleGenerate(repositoryId: string) {
		setGeneration((current) => ({
			generatingRepositoryId: repositoryId,
			errorByRepository: withoutKey(current.errorByRepository, repositoryId),
		}));
		try {
			const suggestion = await commands.aiAssistSuggestPullRequest(
				project.tag,
				context.id,
				repositoryId,
			);
			setDrafts((current) => {
				const existing = current[repositoryId] ?? EMPTY_DRAFT;
				const title = existing.title.trim() ? existing.title : suggestion.title;
				return { ...current, [repositoryId]: { title, description: suggestion.description } };
			});
		} catch (error) {
			const kind: AiAssistErrorKind = isCommandError(error)
				? (error.kind as AiAssistErrorKind)
				: "failed";
			// Cancel leaves the user where they were, silently.
			if (kind === "cancelled") return;
			const message = isCommandError(error) ? error.message : String(error);
			setGeneration((current) => ({
				...current,
				errorByRepository: {
					...current.errorByRepository,
					[repositoryId]: t(generationErrorKey(kind), { message }),
				},
			}));
		} finally {
			setGeneration((current) => ({ ...current, generatingRepositoryId: null }));
		}
	}

	function cancelGeneration() {
		void commands.aiAssistCancelPullRequest();
	}

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
		const isGenerating = generation.generatingRepositoryId === repositoryId;
		const generationError = generation.errorByRepository[repositoryId];
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
							{isGenerating ? (
								<Button
									size="xs"
									variant="light"
									color="red"
									leftSection={<Loader size={12} />}
									onClick={cancelGeneration}
								>
									{t("contexts.createPr.cancelGeneration")}
								</Button>
							) : (
								<Button
									size="xs"
									variant="light"
									leftSection={<SparkleIcon size={14} />}
									disabled={!canCreate}
									onClick={() => handleGenerate(repositoryId)}
								>
									{t("contexts.createPr.generate")}
								</Button>
							)}
						</Group>
						{isGenerating && (
							<Text size="xs" c="dimmed">
								{t("contexts.createPr.generating")}
							</Text>
						)}
						{generationError && (
							<Alert color="red" icon={<WarningIcon size={16} />}>
								{generationError}
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
