import {
	ActionIcon,
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
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { CreateContextParams, RepositoryBranchConfig } from "@/hooks/useContexts";
import type { Context, Project } from "@/models/Project";

type CreateFromContextModalProps = {
	opened: boolean;
	onClose: () => void;
	sourceContext: Context;
	project: Project;
	onCreate: (params: CreateContextParams) => Promise<void>;
};

type RepositoryFormState = {
	createBranch: boolean;
	branchName: string;
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
	const [repoStates, setRepoStates] = useState<Record<string, RepositoryFormState>>(() =>
		Object.fromEntries(
			project.repositories.map((repo) => {
				const sourceBranch = sourceContext.getBranchForRepository(repo.id) ?? "main";
				return [repo.id, { createBranch: true, branchName: "", baseBranch: sourceBranch }];
			}),
		),
	);
	const [loading, setLoading] = useState(false);
	const [branchErrors, setBranchErrors] = useState<Record<string, string>>({});
	const [branchOptions, setBranchOptions] = useState<Record<string, string[]>>({});

	useEffect(() => {
		async function fetchBranches() {
			const options: Record<string, string[]> = {};
			for (const repo of project.repositories) {
				try {
					const resolvedPath = await repo.resolveAbsolutePath(project.path);
					const branches = await invoke<string[]>("list_branches", { path: resolvedPath });
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
	}, [opened, project]);

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

	const hasAtLeastOneBranch = Object.values(repoStates).some(
		(s) => s.createBranch && s.branchName.trim(),
	);

	const hasErrors = Object.keys(branchErrors).length > 0;

	function copyNameToBranches() {
		if (!contextName.trim()) return;
		setRepoStates((prev) => {
			const next = { ...prev };
			for (const id of Object.keys(next)) {
				if (next[id].createBranch) {
					next[id] = { ...next[id], branchName: contextName.trim() };
				}
			}
			return next;
		});
	}

	async function validateBranches(): Promise<boolean> {
		const errors: Record<string, string> = {};

		for (const repo of project.repositories) {
			const state = repoStates[repo.id];
			if (!state.createBranch || !state.branchName.trim()) continue;

			try {
				const resolvedPath = await repo.resolveAbsolutePath(project.path);
				const worktrees = await invoke<{ branch: string | null; path: string }[]>(
					"list_worktrees",
					{ path: resolvedPath },
				);
				const conflicting = worktrees.find((wt) => wt.branch === state.branchName.trim());
				if (conflicting) {
					errors[repo.id] = t("contexts.branchAlreadyInUse", {
						branch: state.branchName.trim(),
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

			const repositories: RepositoryBranchConfig[] = project.repositories.map((repo) => {
				const state = repoStates[repo.id];
				return {
					repositoryId: repo.id,
					createBranch: state.createBranch && !!state.branchName.trim(),
					branchName: state.branchName.trim(),
					baseBranch: state.baseBranch,
				};
			});

			await onCreate({ name: contextName.trim(), repositories, baseContextName: sourceContext.name });
			setContextName("");
			onClose();
		} finally {
			setLoading(false);
		}
	}

	return (
		<Modal opened={opened} onClose={onClose} title={t("contexts.createFromContext")} size="lg">
			<Stack gap="md">
				<Text size="sm" c="dimmed">
					{t("contexts.createFromContextDescription", { name: sourceContext.name })}
				</Text>
				<TextInput
					label={t("contexts.contextName")}
					placeholder="feature/my-feature"
					value={contextName}
					onChange={(e) => setContextName(e.currentTarget.value)}
					required
					rightSection={
						<Tooltip label={t("contexts.copyNameToBranches")}>
							<ActionIcon
								variant="subtle"
								size="sm"
								disabled={!contextName.trim()}
								onClick={copyNameToBranches}
								aria-label={t("contexts.copyNameToBranches")}
							>
								<PaintBrushHouseholdIcon size={16} />
							</ActionIcon>
						</Tooltip>
					}
				/>
				{project.repositories.map((repo) => {
					const state = repoStates[repo.id];
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
									onChange={(e) =>
										updateRepoState(repo.id, { createBranch: e.currentTarget.checked })
									}
								/>
								{state.createBranch && (
									<Group gap="xs" grow>
										<Select
											label={t("contexts.baseBranch")}
											value={state.baseBranch}
											onChange={(value) => updateRepoState(repo.id, { baseBranch: value ?? "" })}
											data={branchOptions[repo.id] ?? []}
											searchable
											size="xs"
										/>
										<TextInput
											label={t("contexts.newBranchName")}
											placeholder="feature/my-feature"
											value={state.branchName}
											onChange={(e) =>
												updateRepoState(repo.id, { branchName: e.currentTarget.value })
											}
											error={branchErrors[repo.id]}
											size="xs"
										/>
									</Group>
								)}
								{!state.createBranch && (
									<Text size="xs" c="dimmed">
										{t("contexts.keepExistingBranch")}: {state.baseBranch}
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
