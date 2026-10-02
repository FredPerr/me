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
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
	CreateContextFromPullRequestsParams,
	PullRequestBranchConfig,
} from "@/hooks/useContexts";
import {
	type RepositoryPullRequests,
	useProjectPullRequests,
} from "@/hooks/useProjectPullRequests";
import type { ProviderPullRequest } from "@/models/git-provider/GitProvider";
import type { Project } from "@/models/Project";
import { fuzzySelectFilter } from "@/utils/fuzzySelectFilter";

type CreateFromPullRequestsModalProps = {
	opened: boolean;
	onClose: () => void;
	project: Project;
	onCreate: (params: CreateContextFromPullRequestsParams) => Promise<void>;
};

type Worktree = { branch: string | null; path: string };

/** Build the Select option list for a repository's open pull requests, keyed by head branch. */
function pullRequestOptions(pullRequests: ProviderPullRequest[]) {
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
	onCreate,
}: CreateFromPullRequestsModalProps) {
	const { t } = useTranslation();
	const projects = useMemo(() => [project], [project]);
	const {
		connected,
		loading: loadingPullRequests,
		groups,
		error,
	} = useProjectPullRequests(projects);

	const [contextName, setContextName] = useState("");
	const [nameEditedManually, setNameEditedManually] = useState(false);
	// Map of repositoryId -> selected PR head branch (null = link the repo).
	const [selectedBranches, setSelectedBranches] = useState<Record<string, string | null>>({});
	const [branchErrors, setBranchErrors] = useState<Record<string, string>>({});
	const [submitError, setSubmitError] = useState<string | null>(null);
	const [loading, setLoading] = useState(false);

	const groupByRepositoryId = useMemo(() => {
		const map: Record<string, RepositoryPullRequests> = {};
		for (const group of groups) {
			map[group.repository.id] = group;
		}
		return map;
	}, [groups]);

	function findBaseBranch(repositoryId: string, headBranch: string): string | null {
		const pullRequest = groupByRepositoryId[repositoryId]?.pullRequests.find(
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
		setBranchErrors((prev) => {
			const next = { ...prev };
			delete next[repositoryId];
			return next;
		});
	}

	const hasSelection = Object.values(selectedBranches).some((branch) => !!branch);
	const hasErrors = Object.keys(branchErrors).length > 0;
	const canSubmit = !!contextName.trim() && hasSelection && !hasErrors;

	async function validateSelectedBranches(): Promise<boolean> {
		const errors: Record<string, string> = {};

		for (const repo of project.effectiveRepositories()) {
			const branch = selectedBranches[repo.id];
			if (!branch) continue;

			try {
				const resolvedPath = await repo.resolveAbsolutePath(project.path);
				const worktrees = await invoke<Worktree[]>("list_worktrees", { path: resolvedPath });
				const conflicting = worktrees.find((worktree) => worktree.branch === branch);
				if (conflicting) {
					errors[repo.id] = t("contexts.branchAlreadyInUse", {
						branch,
						worktree: conflicting.path,
					});
				}
			} catch {
				// Skip validation when worktrees cannot be listed.
			}
		}

		setBranchErrors(errors);
		return Object.keys(errors).length === 0;
	}

	function reset() {
		setContextName("");
		setNameEditedManually(false);
		setSelectedBranches({});
		setBranchErrors({});
		setSubmitError(null);
	}

	async function handleSubmit() {
		if (!canSubmit) return;

		setLoading(true);
		setSubmitError(null);
		try {
			const isValid = await validateSelectedBranches();
			if (!isValid) return;

			const repositories: PullRequestBranchConfig[] = project
				.effectiveRepositories()
				.map((repo) => {
					const headBranch = selectedBranches[repo.id] ?? null;
					return {
						repositoryId: repo.id,
						headBranch,
						baseBranch: headBranch ? findBaseBranch(repo.id, headBranch) : null,
					};
				});

			await onCreate({ name: contextName.trim(), repositories });
			reset();
			onClose();
		} catch (createError) {
			setSubmitError(String(createError));
		} finally {
			setLoading(false);
		}
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

				{!connected ? (
					<Alert color="yellow">{t("contexts.pullRequestsNotConnected")}</Alert>
				) : loadingPullRequests ? (
					<Group justify="center" py="md">
						<Loader size="sm" />
					</Group>
				) : error ? (
					<Alert color="red" title={t("common.error")}>
						{t("contexts.pullRequestsLoadError")}
					</Alert>
				) : (
					<>
						{project.effectiveRepositories().map((repo) => {
							const group = groupByRepositoryId[repo.id];
							const pullRequests = group?.pullRequests ?? [];
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
										error={branchErrors[repo.id]}
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
											{t("contexts.repoWillBeLinked")}
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
					<Button onClick={handleSubmit} loading={loading} disabled={!canSubmit}>
						{t("common.create")}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}
