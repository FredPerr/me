import {
	Alert,
	Badge,
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
import {
	commands,
	type HostedPullRequestView,
	type ProjectView,
	type RepositoryBranchInput,
} from "@/bindings";
import { PullRequestsNotices } from "@/components/shared/PullRequestsNotices";
import { useProjectPullRequests } from "@/hooks/gitHosting/useProjectPullRequests";
import { useBranchNameSuggestion } from "@/hooks/workspace/useBranchNameSuggestion";
import { useCreateContext } from "@/hooks/workspace/useContextMutations";
import { describeError } from "@/infra/commandError";
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

/** Build the Select option list for a repository's open pull requests, keyed by head branch. */
function pullRequestOptions(pullRequests: HostedPullRequestView[]) {
	return pullRequests.map((pullRequest) => ({
		value: pullRequest.headBranch,
		label: `#${pullRequest.number} ${pullRequest.title}`,
	}));
}

/** Derive a default context name from the selected pull requests. */
function deriveContextName(selectedHeadBranches: string[]): string {
	if (selectedHeadBranches.length === 1) return selectedHeadBranches[0];
	return selectedHeadBranches.join("+");
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
	// Map of repositoryId -> selected PR head branch (null = a new branch named after the context).
	const [selectedBranches, setSelectedBranches] = useState<Record<string, string | null>>({});
	const [submitError, setSubmitError] = useState<string | null>(null);
	const [fetching, setFetching] = useState(false);
	const suggestedBranchName = useBranchNameSuggestion(
		project.tag,
		contextName,
		project.branchNaming.defaultPrefix,
	);

	const pullRequestsByRepositoryId = useMemo(() => {
		const map: Record<string, HostedPullRequestView[]> = {};
		for (const repository of pullRequestRepositories ?? []) {
			map[repository.repositoryId] = listedPullRequests(repository);
		}
		return map;
	}, [pullRequestRepositories]);
	const notices = useMemo(
		() => pullRequestsNotices(labeledByRepositoryName(pullRequestRepositories ?? [])),
		[pullRequestRepositories],
	);

	function findBaseBranch(repositoryId: string, headBranch: string): string | null {
		const pullRequest = pullRequestsByRepositoryId[repositoryId]?.find(
			(candidate) => candidate.headBranch === headBranch,
		);
		return pullRequest?.baseBranch ?? null;
	}

	// Keep the context name in sync with the selection until the user types their own.
	useEffect(() => {
		if (nameEditedManually) return;
		const selectedHeadBranches = Object.values(selectedBranches).filter(
			(branch): branch is string => !!branch,
		);
		setContextName(selectedHeadBranches.length > 0 ? deriveContextName(selectedHeadBranches) : "");
	}, [selectedBranches, nameEditedManually]);

	function selectBranch(repositoryId: string, branch: string | null) {
		setSelectedBranches((prev) => ({ ...prev, [repositoryId]: branch }));
	}

	const hasSelection = Object.values(selectedBranches).some((branch) => !!branch);
	const branchChoices: RepositoryBranchInput[] = repositories.map((repository) => {
		const headBranch = selectedBranches[repository.id];
		const branch = headBranch
			? { kind: "existingBranch" as const, name: headBranch }
			: {
					kind: "newBranch" as const,
					name: suggestedBranchName,
					baseBranch: findDefaultBranch(project, repository.id) ?? "",
				};
		return { repositoryId: repository.id, branch };
	});
	const canSubmit =
		!!contextName.trim() &&
		hasSelection &&
		branchChoices.every(
			({ branch }) =>
				branch.name !== "" && (branch.kind !== "newBranch" || branch.baseBranch !== ""),
		);

	/** Open pull requests usually live only on the remote: fetch their branches first. */
	async function fetchSelectedBranches(): Promise<boolean> {
		setFetching(true);
		try {
			for (const repository of repositories) {
				const headBranch = selectedBranches[repository.id];
				if (headBranch) await commands.fetchPrBranch(repository.path, headBranch);
			}
			return true;
		} catch (fetchError) {
			setSubmitError(describeError(fetchError));
			return false;
		} finally {
			setFetching(false);
		}
	}

	function reset() {
		setContextName("");
		setNameEditedManually(false);
		setSelectedBranches({});
		setSubmitError(null);
	}

	async function handleSubmit() {
		if (!canSubmit) return;
		setSubmitError(null);
		if (!(await fetchSelectedBranches())) return;
		try {
			await createContext.mutateAsync({
				projectTag: project.tag,
				name: contextName.trim(),
				baseContextId: null,
				origin: { kind: "fromPullRequests" },
				status: "review",
				agentPrompt: null,
				branches: branchChoices,
			});
		} catch {
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
						{repositories.map((repo) => {
							const pullRequests = pullRequestsByRepositoryId[repo.id] ?? [];
							const branch = selectedBranches[repo.id] ?? null;

							return (
								<Box
									key={repo.id}
									p="xs"
									style={{
										border: "1px solid var(--mantine-color-default-border)",
										borderRadius: "var(--mantine-radius-sm)",
									}}
								>
									<Select
										label={repo.name}
										placeholder={
											pullRequests.length === 0
												? t("contexts.noOpenPullRequests")
												: t("contexts.selectPullRequest")
										}
										value={branch}
										onChange={(value) => selectBranch(repo.id, value)}
										data={pullRequestOptions(pullRequests)}
										nothingFoundMessage={t("contexts.noOpenPullRequests")}
										disabled={pullRequests.length === 0}
										searchable
										clearable
										filter={fuzzySelectFilter}
										size="xs"
									/>
									{branch ? (
										<Group gap="xs" mt={4}>
											<Badge variant="light" size="xs">
												{branch}
											</Badge>
											<Text size="xs" c="dimmed">
												{t("contexts.pullRequestBaseBranch", {
													base: findBaseBranch(repo.id, branch) ?? "—",
												})}
											</Text>
										</Group>
									) : (
										<Text size="xs" c="dimmed" mt={4}>
											{t("contexts.repoGetsNewBranch", {
												branch: suggestedBranchName || t("contexts.branchFromContextName"),
												base: findDefaultBranch(project, repo.id) ?? "—",
											})}
										</Text>
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
					<Button
						onClick={handleSubmit}
						loading={fetching || createContext.isPending}
						disabled={!canSubmit}
					>
						{t("common.create")}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}
