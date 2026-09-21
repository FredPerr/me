import {
	ActionIcon,
	Box,
	Button,
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
import type { CreateContextFromBranchesParams, ExistingBranchConfig } from "@/hooks/useContexts";
import type { Project } from "@/models/Project";

type CreateFromBranchesModalProps = {
	opened: boolean;
	onClose: () => void;
	project: Project;
	onCreate: (params: CreateContextFromBranchesParams) => Promise<void>;
};

type Worktree = { branch: string | null; path: string };

export function CreateFromBranchesModal({
	opened,
	onClose,
	project,
	onCreate,
}: CreateFromBranchesModalProps) {
	const { t } = useTranslation();
	const [contextName, setContextName] = useState("");
	const [selectedBranches, setSelectedBranches] = useState<Record<string, string | null>>({});
	const [branchOptions, setBranchOptions] = useState<Record<string, string[]>>({});
	const [branchErrors, setBranchErrors] = useState<Record<string, string>>({});
	const [loading, setLoading] = useState(false);

	useEffect(() => {
		if (!opened) return;

		async function fetchBranches() {
			const options: Record<string, string[]> = {};
			for (const repo of project.repositories) {
				try {
					const resolvedPath = await repo.resolveAbsolutePath(project.path);
					options[repo.id] = await invoke<string[]>("list_branches", { path: resolvedPath });
				} catch {
					options[repo.id] = [];
				}
			}
			setBranchOptions(options);
		}

		fetchBranches();
	}, [opened, project]);

	function selectBranch(repositoryId: string, branch: string | null) {
		setSelectedBranches((prev) => ({ ...prev, [repositoryId]: branch }));
		setBranchErrors((prev) => {
			const next = { ...prev };
			delete next[repositoryId];
			return next;
		});
	}

	function copyBranchToName(repositoryId: string) {
		const branch = selectedBranches[repositoryId];
		if (branch) {
			setContextName(branch);
		}
	}

	const selectedCount = Object.values(selectedBranches).filter(Boolean).length;
	const hasErrors = Object.keys(branchErrors).length > 0;
	const canSubmit = !!contextName.trim() && selectedCount > 0 && !hasErrors;

	async function validateSelectedBranches(): Promise<boolean> {
		const errors: Record<string, string> = {};

		for (const repo of project.repositories) {
			const branch = selectedBranches[repo.id];
			if (!branch) continue;

			try {
				const resolvedPath = await repo.resolveAbsolutePath(project.path);
				const worktrees = await invoke<Worktree[]>("list_worktrees", { path: resolvedPath });
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

	function reset() {
		setContextName("");
		setSelectedBranches({});
		setBranchErrors({});
	}

	async function handleSubmit() {
		if (!canSubmit) return;

		setLoading(true);
		try {
			const isValid = await validateSelectedBranches();
			if (!isValid) return;

			const repositories: ExistingBranchConfig[] = project.repositories.map((repo) => ({
				repositoryId: repo.id,
				branch: selectedBranches[repo.id] ?? null,
			}));

			await onCreate({ name: contextName.trim(), repositories });
			reset();
			onClose();
		} finally {
			setLoading(false);
		}
	}

	function handleClose() {
		reset();
		onClose();
	}

	return (
		<Modal opened={opened} onClose={handleClose} title={t("contexts.createFromBranches")} size="lg">
			<Stack gap="md">
				<Text size="sm" c="dimmed">
					{t("contexts.createFromBranchesDescription")}
				</Text>
				<TextInput
					label={t("contexts.contextName")}
					placeholder="feature/my-feature"
					value={contextName}
					onChange={(e) => setContextName(e.currentTarget.value)}
					required
				/>
				{project.repositories.map((repo) => {
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
							<Group gap="xs" align="flex-end" wrap="nowrap">
								<Select
									label={repo.name}
									placeholder={t("contexts.selectExistingBranch")}
									value={branch}
									onChange={(value) => selectBranch(repo.id, value)}
									data={branchOptions[repo.id] ?? []}
									error={branchErrors[repo.id]}
									searchable
									clearable
									size="xs"
									style={{ flex: 1 }}
								/>
								<Tooltip label={t("contexts.copyBranchToName")}>
									<ActionIcon
										variant="subtle"
										size="lg"
										onClick={() => copyBranchToName(repo.id)}
										disabled={!branch}
										aria-label={t("contexts.copyBranchToName")}
									>
										<PaintBrushHouseholdIcon size={16} />
									</ActionIcon>
								</Tooltip>
							</Group>
							{!branch && (
								<Text size="xs" c="dimmed" mt={4}>
									{t("contexts.repoWillBeLinked")}
								</Text>
							)}
						</Box>
					);
				})}
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
