import {
	ActionIcon,
	Autocomplete,
	Box,
	Button,
	Checkbox,
	Group,
	Modal,
	Select,
	Stack,
	Text,
	TextInput,
	Tooltip,
} from "@mantine/core";
import { PaintBrushHouseholdIcon } from "@phosphor-icons/react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import type { CreateContextParams, RepositoryBranchConfig } from "@/hooks/useContexts";
import type { Context, Project } from "@/models/Project";
import { fuzzySelectFilter } from "@/utils/fuzzySelectFilter";
import { BranchNameFromTaskPopover } from "./BranchNameFromTaskPopover";

type CreateFromContextModalProps = {
	opened: boolean;
	onClose: () => void;
	sourceContext: Context;
	project: Project;
	onCreate: (params: CreateContextParams) => Promise<void>;
};

type RepositoryFormState = {
	/** When off, the repo is symlinked to the base context instead of branched. */
	createBranch: boolean;
	/** Branch to check out: an existing branch, or a new name to create. */
	branchName: string;
	/** Base branch a new branch is cut from (ignored when the branch exists). */
	baseBranch: string;
};

export function CreateFromContextModal({
	opened,
	onClose,
	sourceContext,
	project,
	onCreate,
}: CreateFromContextModalProps) {
	const { t } = useTranslation();
	const [contextName, setContextName] = useState("");
	const repositories = useMemo(() => project.effectiveRepositories(), [project]);
	const [repoStates, setRepoStates] = useState<Record<string, RepositoryFormState>>(() =>
		Object.fromEntries(
			repositories.map((repo) => {
				const sourceBranch = sourceContext.getBranchForRepository(repo.id) ?? "main";
				return [repo.id, { createBranch: true, branchName: "", baseBranch: sourceBranch }];
			}),
		),
	);
	const [loading, setLoading] = useState(false);
	const [syncBranchNames, setSyncBranchNames] = useState(true);
	const [isBranchPopoverOpened, setIsBranchPopoverOpened] = useState(false);
	const [branchErrors, setBranchErrors] = useState<Record<string, string>>({});
	const [branchOptions, setBranchOptions] = useState<Record<string, string[]>>({});

	useEffect(() => {
		async function fetchBranches() {
			const options: Record<string, string[]> = {};
			for (const repo of repositories) {
				try {
					const resolvedPath = await repo.resolveAbsolutePath(project.path);
					const branches = await commands.listBranches(resolvedPath);
					options[repo.id] = branches;
				} catch {
					options[repo.id] = [];
				}
			}
			setBranchOptions(options);
		}
		if (opened) {
			fetchBranches();
		}
	}, [opened, project, repositories]);

	useEffect(() => {
		if (!opened) setIsBranchPopoverOpened(false);
	}, [opened]);

	function updateRepoState(repositoryId: string, patch: Partial<RepositoryFormState>) {
		setRepoStates((prev) => ({
			...prev,
			[repositoryId]: { ...prev[repositoryId], ...patch },
		}));
		if (patch.branchName !== undefined) {
			setBranchErrors((prev) => {
				const next = { ...prev };
				delete next[repositoryId];
				return next;
			});
		}
	}

	// A repo contributes if it creates/checks out a branch, or is symlinked
	// (createBranch off); so at least one repo always produces a worktree entry.
	const hasAtLeastOneBranch = Object.values(repoStates).some(
		(s) => !s.createBranch || Boolean(s.branchName.trim()),
	);

	const hasErrors = Object.keys(branchErrors).length > 0;

	/** Applies one branch name to every repo that is set to create a branch. */
	function syncBranchesToName(name: string) {
		const trimmed = name.trim();
		setRepoStates((prev) => {
			const next = { ...prev };
			for (const id of Object.keys(next)) {
				if (next[id].createBranch) {
					next[id] = { ...next[id], branchName: trimmed };
				}
			}
			return next;
		});
	}

	function handleContextNameChange(name: string) {
		setContextName(name);
		if (syncBranchNames) {
			syncBranchesToName(name);
		}
	}

	function applyGeneratedBranchName(branchName: string) {
		if (syncBranchNames) {
			handleContextNameChange(branchName);
			return;
		}
		for (const repositoryId of Object.keys(repoStates)) {
			if (repoStates[repositoryId].createBranch) {
				updateRepoState(repositoryId, { branchName });
			}
		}
		if (!contextName.trim()) {
			setContextName(branchName);
		}
	}

	function toggleSync() {
		const nextEnabled = !syncBranchNames;
		setSyncBranchNames(nextEnabled);
		if (nextEnabled) {
			syncBranchesToName(contextName);
		}
	}

	async function validateBranches(): Promise<boolean> {
		const errors: Record<string, string> = {};

		for (const repo of repositories) {
			const state = repoStates[repo.id];
			const branch = state.branchName.trim();
			if (!state.createBranch || !branch) continue;

			// Selecting an existing branch is fine: the backend checks it out
			// (reusing its worktree if any). Only a brand-new branch name that
			// collides with an already checked-out branch is a real conflict.
			const branchExists = (branchOptions[repo.id] ?? []).includes(branch);
			if (branchExists) continue;

			try {
				const resolvedPath = await repo.resolveAbsolutePath(project.path);
				const worktrees = await commands.listWorktrees(resolvedPath);
				const conflicting = worktrees.find((wt) => wt.branch === branch);
				if (conflicting) {
					errors[repo.id] = t("contexts.branchAlreadyInUse", {
						branch,
						worktree: conflicting.path,
					});
				}
			} catch {
				// skip validation if we can't list worktrees
			}
		}

		setBranchErrors(errors);
		return Object.keys(errors).length === 0;
	}

	async function handleSubmit() {
		if (!contextName.trim() || !hasAtLeastOneBranch) return;

		setLoading(true);
		try {
			const isValid = await validateBranches();
			if (!isValid) return;

			const repositoryConfigs: RepositoryBranchConfig[] = repositories.map((repo) => {
				const state = repoStates[repo.id];
				const branch = state.branchName.trim();
				const branchExists = (branchOptions[repo.id] ?? []).includes(branch);
				// Checkbox off, or no branch typed: link this repo to the base
				// context's checkout. Empty branchName drives the link path in the
				// hook; baseBranch keeps the source branch so the linked repo still
				// records a meaningful branch. An existing branch is checked out
				// as-is; a new name is created off the base branch.
				if (!state.createBranch || !branch) {
					return {
						repositoryId: repo.id,
						createBranch: false,
						branchName: "",
						baseBranch: state.baseBranch,
					};
				}
				return {
					repositoryId: repo.id,
					createBranch: !branchExists,
					branchName: branch,
					baseBranch: branchExists ? branch : state.baseBranch,
				};
			});

			await onCreate({
				name: contextName.trim(),
				repositories: repositoryConfigs,
				baseContextName: sourceContext.name,
			});
			setContextName("");
			onClose();
		} finally {
			setLoading(false);
		}
	}

	return (
		<Modal
			opened={opened}
			onClose={onClose}
			closeOnEscape={!isBranchPopoverOpened}
			title={t("contexts.createFromContext")}
		>
			<Stack gap="md">
				<Text size="sm" c="dimmed">
					{t("contexts.createFromContextDescription", { name: sourceContext.name })}
				</Text>
				<TextInput
					label={t("contexts.contextName")}
					placeholder="feature/my-feature"
					value={contextName}
					onChange={(e) => handleContextNameChange(e.currentTarget.value)}
					required
					rightSectionWidth={60}
					rightSection={
						<Group gap={4} wrap="nowrap">
							<BranchNameFromTaskPopover
								branchNaming={project.branchNaming}
								opened={isBranchPopoverOpened}
								onOpenedChange={setIsBranchPopoverOpened}
								onApply={applyGeneratedBranchName}
							/>
							<Tooltip
								label={
									syncBranchNames ? t("contexts.syncBranchesOn") : t("contexts.syncBranchesOff")
								}
							>
								<ActionIcon
									variant={syncBranchNames ? "filled" : "subtle"}
									size="sm"
									onClick={toggleSync}
									aria-label={t("contexts.syncBranchNames")}
									aria-pressed={syncBranchNames}
								>
									<PaintBrushHouseholdIcon size={16} />
								</ActionIcon>
							</Tooltip>
						</Group>
					}
				/>
				{repositories.map((repo) => {
					const state = repoStates[repo.id];
					const branches = branchOptions[repo.id] ?? [];
					const branch = state.branchName.trim();
					const branchExists = branches.includes(branch);
					return (
						<Box
							key={repo.id}
							p="xs"
							style={{
								border: "1px solid var(--mantine-color-default-border)",
								borderRadius: "var(--mantine-radius-sm)",
							}}
						>
							<Stack gap="xs">
								<Checkbox
									label={`${t("contexts.createNewBranch")} — ${repo.name}`}
									checked={state.createBranch}
									onChange={(e) => {
										const checked = e.currentTarget.checked;
										updateRepoState(repo.id, {
											createBranch: checked,
											...(checked && syncBranchNames ? { branchName: contextName.trim() } : {}),
										});
									}}
								/>
								{state.createBranch ? (
									<Group gap="xs" grow align="flex-start">
										<Select
											label={t("contexts.baseBranch")}
											value={state.baseBranch || null}
											onChange={(value) => updateRepoState(repo.id, { baseBranch: value ?? "" })}
											data={branches}
											placeholder={t("contexts.selectExistingBranch")}
											nothingFoundMessage={t("contexts.noBranchesFound")}
											disabled={branchExists}
											searchable
											size="xs"
										/>
										<Autocomplete
											label={t("contexts.contextBranch")}
											description={
												branch
													? branchExists
														? t("contexts.branchExistsHint")
														: t("contexts.branchNewHint")
													: undefined
											}
											placeholder={t("contexts.selectExistingBranch")}
											value={state.branchName}
											onChange={(value) => updateRepoState(repo.id, { branchName: value })}
											data={branches}
											readOnly={syncBranchNames}
											error={branchErrors[repo.id]}
											filter={fuzzySelectFilter}
											size="xs"
										/>
									</Group>
								) : (
									<Text size="xs" c="dimmed">
										{t("contexts.repoWillBeLinkedToBase")}
									</Text>
								)}
							</Stack>
						</Box>
					);
				})}
				<Group justify="flex-end">
					<Button variant="subtle" onClick={onClose}>
						{t("common.cancel")}
					</Button>
					<Button
						onClick={handleSubmit}
						loading={loading}
						disabled={!contextName.trim() || !hasAtLeastOneBranch || hasErrors}
					>
						{t("common.create")}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}
