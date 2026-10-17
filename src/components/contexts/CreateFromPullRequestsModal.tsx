import {
	Alert,
	Box,
	Button,
	Group,
	Loader,
	Modal,
	Select,
	Stack,
	Text,
	TextInput,
} from "@mantine/core";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
	BranchChoiceInput,
	HostedPullRequestView,
	ProjectView,
	RepositoryBranchInput,
} from "@/bindings";
import { PullRequestsNotices } from "@/components/shared/PullRequestsNotices";
import { useProjectPullRequests } from "@/hooks/gitHosting/useProjectPullRequests";
import { useBranchNameSuggestion } from "@/hooks/workspace/useBranchNameSuggestion";
import { useCreateContext } from "@/hooks/workspace/useContextMutations";
import { describeError } from "@/infra/commandError";
import {
	groupByHeadBranch,
	selectionForGroup,
	suggestedName,
} from "@/models/git-hosting/pullRequestGroups";
import {
	labeledByRepositoryName,
	listedPullRequests,
	pullRequestsNotices,
} from "@/models/git-hosting/repositoryPullRequests";
import { findDefaultBranch } from "@/models/projectView";
import { fuzzySelectFilter } from "@/utils/fuzzySelectFilter";

type CreateFromPullRequestsModalProps = {
	opened: boolean;
	onClose: () => void;
	project: ProjectView;
};

/** A repository's open pull requests, by head branch, with its host string for the create call. */
type RepositoryPullRequestData = {
	pullRequests: HostedPullRequestView[];
	host: string | null;
};

const GROUP_PULL_REQUEST_SEPARATOR = ", ";

/** The common base branch of the selected pull requests when they all share one. */
function commonBaseBranch(selectedPullRequests: HostedPullRequestView[]): string | null {
	if (selectedPullRequests.length === 0) return null;
	const baseBranches = new Set(selectedPullRequests.map((pullRequest) => pullRequest.baseBranch));
	return baseBranches.size === 1 ? [...baseBranches][0] : null;
}

export function CreateFromPullRequestsModal({
	opened,
	onClose,
	project,
}: CreateFromPullRequestsModalProps) {
	const { t } = useTranslation();
	const createContext = useCreateContext();
	const repositories = project.effectiveRepositories;
	const projectPullRequests = useProjectPullRequests(project.tag);
	const pullRequestRepositories = projectPullRequests.data;

	const [contextName, setContextName] = useState("");
	const [nameEditedManually, setNameEditedManually] = useState(false);
	const [selectedGroupHeadBranch, setSelectedGroupHeadBranch] = useState<string | null>(null);
	// repositoryId -> selected pull request head branch (absent = a new branch named after the context).
	const [selectedBranches, setSelectedBranches] = useState<Record<string, string>>({});
	// repositoryId -> edited new-branch base branch, overriding the suggested base.
	const [baseOverrides, setBaseOverrides] = useState<Record<string, string>>({});
	const [submitError, setSubmitError] = useState<string | null>(null);

	const suggestedBranchName = useBranchNameSuggestion(
		project.tag,
		contextName,
		project.branchNaming.defaultPrefix,
	);

	const pullRequestDataByRepositoryId = useMemo(() => {
		const map: Record<string, RepositoryPullRequestData> = {};
		for (const repository of pullRequestRepositories ?? []) {
			map[repository.repositoryId] = {
				pullRequests: listedPullRequests(repository),
				host: repository.host,
			};
		}
		return map;
	}, [pullRequestRepositories]);

	const groups = useMemo(
		() => groupByHeadBranch(pullRequestRepositories ?? []),
		[pullRequestRepositories],
	);
	const notices = useMemo(
		() => pullRequestsNotices(labeledByRepositoryName(pullRequestRepositories ?? [])),
		[pullRequestRepositories],
	);

	function findPullRequest(
		repositoryId: string,
		headBranch: string,
	): HostedPullRequestView | undefined {
		return pullRequestDataByRepositoryId[repositoryId]?.pullRequests.find(
			(pullRequest) => pullRequest.headBranch === headBranch,
		);
	}

	const selectedPullRequests = repositories.flatMap((repository) => {
		const headBranch = selectedBranches[repository.id];
		const pullRequest = headBranch ? findPullRequest(repository.id, headBranch) : undefined;
		return pullRequest ? [pullRequest] : [];
	});
	const selectedBaseBranch = commonBaseBranch(selectedPullRequests);

	// Keep the context name in sync with the selection until the user types their own.
	const suggestedContextName = suggestedName(selectedPullRequests);
	useEffect(() => {
		if (nameEditedManually) return;
		setContextName(suggestedContextName);
	}, [suggestedContextName, nameEditedManually]);

	function newBranchBase(repositoryId: string): string {
		return (
			baseOverrides[repositoryId] ??
			selectedBaseBranch ??
			findDefaultBranch(project, repositoryId) ??
			""
		);
	}

	function applyGroup(headBranch: string | null) {
		setSelectedGroupHeadBranch(headBranch);
		if (headBranch === null) return;
		const group = groups.find((candidate) => candidate.headBranch === headBranch);
		if (group) setSelectedBranches(selectionForGroup(group));
	}

	function selectBranch(repositoryId: string, headBranch: string | null) {
		setSelectedBranches((previous) => {
			const next = { ...previous };
			if (headBranch === null) delete next[repositoryId];
			else next[repositoryId] = headBranch;
			return next;
		});
	}

	function setBaseOverride(repositoryId: string, baseBranch: string) {
		setBaseOverrides((previous) => ({ ...previous, [repositoryId]: baseBranch }));
	}

	function buildBranch(repository: { id: string }): BranchChoiceInput {
		const headBranch = selectedBranches[repository.id];
		const pullRequest = headBranch ? findPullRequest(repository.id, headBranch) : undefined;
		const host = pullRequestDataByRepositoryId[repository.id]?.host;
		if (pullRequest && host) {
			return {
				kind: "pullRequest",
				headBranch: pullRequest.headBranch,
				baseBranch: pullRequest.baseBranch,
				host,
				number: pullRequest.number,
				url: pullRequest.url,
			};
		}
		return {
			kind: "newBranch",
			name: suggestedBranchName,
			baseBranch: newBranchBase(repository.id),
		};
	}

	const branches: RepositoryBranchInput[] = repositories.map((repository) => ({
		repositoryId: repository.id,
		branch: buildBranch(repository),
	}));
	const hasSelection = branches.some(({ branch }) => branch.kind === "pullRequest");
	const canSubmit =
		!!contextName.trim() &&
		hasSelection &&
		branches.every(
			({ branch }) =>
				branch.kind !== "newBranch" || (branch.name !== "" && branch.baseBranch !== ""),
		);

	function reset() {
		setContextName("");
		setNameEditedManually(false);
		setSelectedGroupHeadBranch(null);
		setSelectedBranches({});
		setBaseOverrides({});
		setSubmitError(null);
	}

	async function handleSubmit() {
		if (!canSubmit) return;
		setSubmitError(null);
		try {
			await createContext.mutateAsync({
				projectTag: project.tag,
				name: contextName.trim(),
				baseContextId: null,
				origin: { kind: "fromPullRequests" },
				status: "review",
				agentPrompt: null,
				branches,
			});
		} catch (error) {
			setSubmitError(describeError(error));
			return;
		}
		reset();
		onClose();
	}

	function handleClose() {
		reset();
		onClose();
	}

	return (
		<Modal opened={opened} onClose={handleClose} title={t("contexts.createFromPullRequests")}>
			<Stack gap="md">
				<Text size="sm" c="dimmed">
					{t("contexts.createFromPullRequestsDescription")}
				</Text>

				{projectPullRequests.isPending ? (
					<Group justify="center" py="md">
						<Loader size="sm" />
					</Group>
				) : projectPullRequests.isError ? (
					<Alert color="red" title={t("common.error")}>
						{t("contexts.pullRequestsLoadError")} {describeError(projectPullRequests.error)}
					</Alert>
				) : (
					<>
						<PullRequestsNotices notices={notices} />

						<Select
							label={t("contexts.pullRequestGroupLabel")}
							placeholder={
								groups.length === 0
									? t("contexts.noOpenPullRequests")
									: t("contexts.pullRequestGroupPlaceholder")
							}
							value={selectedGroupHeadBranch}
							onChange={applyGroup}
							data={groups.map((group) => ({
								value: group.headBranch,
								label: t("contexts.pullRequestGroupOption", {
									headBranch: group.headBranch,
									pullRequests: group.items
										.map((item) => `${item.repositoryName} #${item.pullRequest.number}`)
										.join(GROUP_PULL_REQUEST_SEPARATOR),
								}),
							}))}
							nothingFoundMessage={t("contexts.noOpenPullRequests")}
							disabled={groups.length === 0}
							searchable
							clearable
							filter={fuzzySelectFilter}
						/>

						{repositories.map((repository) => {
							const repositoryData = pullRequestDataByRepositoryId[repository.id];
							const pullRequests = repositoryData?.pullRequests ?? [];
							const headBranch = selectedBranches[repository.id] ?? null;

							return (
								<Box
									key={repository.id}
									p="xs"
									style={{
										border: "1px solid var(--mantine-color-default-border)",
										borderRadius: "var(--mantine-radius-sm)",
									}}
								>
									<Select
										label={repository.name}
										placeholder={
											pullRequests.length === 0
												? t("contexts.noOpenPullRequests")
												: t("contexts.selectPullRequest")
										}
										value={headBranch}
										onChange={(value) => selectBranch(repository.id, value)}
										data={pullRequests.map((pullRequest) => ({
											value: pullRequest.headBranch,
											label: pullRequest.isFork
												? t("contexts.pullRequestForkOption", {
														number: pullRequest.number,
														title: pullRequest.title,
													})
												: `#${pullRequest.number} ${pullRequest.title}`,
											disabled: pullRequest.isFork,
										}))}
										nothingFoundMessage={t("contexts.noOpenPullRequests")}
										disabled={pullRequests.length === 0}
										searchable
										clearable
										filter={fuzzySelectFilter}
										size="xs"
									/>
									{headBranch === null && (
										<TextInput
											mt={4}
											size="xs"
											label={t("contexts.newBranchBaseLabel", {
												branch: suggestedBranchName || t("contexts.branchFromContextName"),
											})}
											value={newBranchBase(repository.id)}
											onChange={(event) =>
												setBaseOverride(repository.id, event.currentTarget.value)
											}
										/>
									)}
								</Box>
							);
						})}

						<TextInput
							label={t("contexts.contextName")}
							placeholder="feature/my-feature"
							value={contextName}
							onChange={(event) => {
								setNameEditedManually(true);
								setContextName(event.currentTarget.value);
							}}
							required
						/>

						{submitError && (
							<Alert color="red" title={t("common.error")}>
								{submitError}
							</Alert>
						)}
					</>
				)}

				<Group justify="flex-end">
					<Button variant="subtle" onClick={handleClose}>
						{t("common.cancel")}
					</Button>
					<Button onClick={handleSubmit} loading={createContext.isPending} disabled={!canSubmit}>
						{t("common.create")}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}
