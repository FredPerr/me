import {
	ActionIcon,
	Badge,
	Box,
	Button,
	Checkbox,
	Divider,
	Group,
	Loader,
	Modal,
	ScrollArea,
	Stack,
	Text,
	Tooltip,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { ArrowClockwiseIcon, BroomIcon, CloudArrowDownIcon } from "@phosphor-icons/react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useErrorModal } from "@/components/shared/ErrorModalProvider";
import { type BranchSelection, type LocalBranch, useLocalBranches } from "@/hooks/useLocalBranches";
import { usePullContexts } from "@/hooks/usePullContexts";
import {
	useRepositoryWorktrees,
	type Worktree,
	type WorktreeSelection,
} from "@/hooks/useRepositoryWorktrees";
import type { Project } from "@/models/Project";

type RepositoryBranchesModalProps = {
	opened: boolean;
	onClose: () => void;
	project: Project;
};

function branchKey(resolvedPath: string, branchName: string): string {
	return `${resolvedPath}:${branchName}`;
}

function isDeletable(branch: LocalBranch): boolean {
	return !branch.isCurrent && !branch.isCheckedOut;
}

function isDead(branch: LocalBranch): boolean {
	return isDeletable(branch) && (branch.upstreamGone || branch.isMerged);
}

function worktreeKey(resolvedPath: string, worktreePath: string): string {
	return `${resolvedPath}:${worktreePath}`;
}

export function RepositoryBranchesModal({
	opened,
	onClose,
	project,
}: RepositoryBranchesModalProps) {
	const { t } = useTranslation();
	const { showError } = useErrorModal();
	const { pullAll, pulling } = usePullContexts(project);
	const { repositoryBranches, loading, reload, deleteBranches, pruneWorktrees } = useLocalBranches(
		project,
		opened,
	);
	const {
		repositoryWorktrees,
		loading: loadingWorktrees,
		reload: reloadWorktrees,
		deleteWorktrees,
	} = useRepositoryWorktrees(project, opened);
	const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());
	const [deleting, setDeleting] = useState(false);
	const [pruning, setPruning] = useState(false);
	const [selectedWorktreeKeys, setSelectedWorktreeKeys] = useState<Set<string>>(new Set());
	const [deletingWorktrees, setDeletingWorktrees] = useState(false);

	const selectionByKey = useMemo(() => {
		const map = new Map<string, BranchSelection>();
		for (const { resolvedPath, branches } of repositoryBranches) {
			for (const branch of branches) {
				if (isDeletable(branch)) {
					map.set(branchKey(resolvedPath, branch.name), {
						resolvedPath,
						branchName: branch.name,
					});
				}
			}
		}
		return map;
	}, [repositoryBranches]);

	// Drop selections that no longer exist (e.g. after a reload).
	const validSelectedKeys = useMemo(
		() => [...selectedKeys].filter((key) => selectionByKey.has(key)),
		[selectedKeys, selectionByKey],
	);

	function toggle(key: string) {
		setSelectedKeys((current) => {
			const next = new Set(current);
			if (next.has(key)) {
				next.delete(key);
			} else {
				next.add(key);
			}
			return next;
		});
	}

	function selectAllInRepo(resolvedPath: string, branches: LocalBranch[]) {
		const deletableKeys = branches
			.filter(isDeletable)
			.map((branch) => branchKey(resolvedPath, branch.name));
		if (deletableKeys.length === 0) return;

		setSelectedKeys((current) => {
			const allSelected = deletableKeys.every((key) => current.has(key));
			const next = new Set(current);
			for (const key of deletableKeys) {
				if (allSelected) {
					next.delete(key);
				} else {
					next.add(key);
				}
			}
			return next;
		});
	}

	async function handlePull() {
		await pullAll();
		setSelectedKeys(new Set());
		await reload();
	}

	async function handlePruneWorktrees() {
		setPruning(true);
		try {
			await pruneWorktrees();
			await reloadWorktrees();
			notifications.show({
				title: t("branches.prunedTitle"),
				message: t("branches.prunedMessage"),
				color: "green",
			});
		} finally {
			setPruning(false);
		}
	}

	const worktreeSelectionByKey = useMemo(() => {
		const map = new Map<string, WorktreeSelection>();
		for (const { resolvedPath, worktrees } of repositoryWorktrees) {
			for (const worktree of worktrees) {
				if (worktree.isDefault) continue;
				map.set(worktreeKey(resolvedPath, worktree.path), {
					repositoryPath: resolvedPath,
					worktreePath: worktree.path,
				});
			}
		}
		return map;
	}, [repositoryWorktrees]);

	const validSelectedWorktreeKeys = useMemo(
		() => [...selectedWorktreeKeys].filter((key) => worktreeSelectionByKey.has(key)),
		[selectedWorktreeKeys, worktreeSelectionByKey],
	);

	function toggleWorktree(key: string) {
		setSelectedWorktreeKeys((current) => {
			const next = new Set(current);
			if (next.has(key)) {
				next.delete(key);
			} else {
				next.add(key);
			}
			return next;
		});
	}

	async function handleDeleteSelectedWorktrees() {
		const selections = validSelectedWorktreeKeys
			.map((key) => worktreeSelectionByKey.get(key))
			.filter((selection): selection is WorktreeSelection => !!selection);

		if (selections.length === 0) return;

		setDeletingWorktrees(true);
		try {
			const failures = await deleteWorktrees(selections);
			const deletedCount = selections.length - failures.length;

			if (deletedCount > 0) {
				notifications.show({
					title: t("worktrees.deletedTitle"),
					message: t("worktrees.deletedCountMessage", { count: deletedCount }),
					color: "green",
				});
			}

			if (failures.length > 0) {
				showError({
					title: t("worktrees.deleteFailedTitle"),
					description: t("worktrees.deleteFailedCountMessage", { count: failures.length }),
					details: failures
						.map((failure) => `• ${failure.selection.worktreePath}: ${failure.message}`)
						.join("\n"),
					hint: t("worktrees.deleteFailedHint"),
				});
			}

			setSelectedWorktreeKeys(new Set());
			await reload();
		} finally {
			setDeletingWorktrees(false);
		}
	}

	async function handleDeleteSelected() {
		const selections = validSelectedKeys
			.map((key) => selectionByKey.get(key))
			.filter((selection): selection is BranchSelection => !!selection);

		if (selections.length === 0) return;

		setDeleting(true);
		try {
			const failures = await deleteBranches(selections);
			const deletedCount = selections.length - failures.length;

			if (deletedCount > 0) {
				notifications.show({
					title: t("branches.deletedTitle"),
					message: t("branches.deletedCountMessage", { count: deletedCount }),
					color: "green",
				});
			}

			if (failures.length > 0) {
				showError({
					title: t("branches.deleteFailedTitle"),
					description: t("branches.deleteFailedCountMessage", { count: failures.length }),
					details: failures
						.map((failure) => `• ${failure.selection.branchName}: ${failure.message}`)
						.join("\n"),
					hint: t("branches.deleteFailedHint"),
				});
			}

			setSelectedKeys(new Set());
		} finally {
			setDeleting(false);
		}
	}

	return (
		<Modal opened={opened} onClose={onClose} title={t("branches.title")} size="lg">
			<Stack gap="md">
				<Group justify="space-between" align="flex-start" wrap="nowrap">
					<Box>
						<Text fw={500}>{t("branches.pullTitle")}</Text>
						<Text size="sm" c="dimmed">
							{t("branches.pullDescription")}
						</Text>
					</Box>
					<Button
						size="xs"
						leftSection={<CloudArrowDownIcon size={13} />}
						loading={pulling}
						onClick={handlePull}
					>
						{t("project.pullContexts")}
					</Button>
				</Group>

				<Divider />

				<Group justify="space-between" align="center">
					<Box>
						<Text fw={500}>{t("branches.deadTitle")}</Text>
						<Text size="sm" c="dimmed">
							{t("branches.deadDescription")}
						</Text>
					</Box>
					<Group gap="xs">
						<Button
							variant="subtle"
							size="compact-xs"
							leftSection={<BroomIcon size={13} />}
							loading={pruning}
							onClick={handlePruneWorktrees}
						>
							{t("branches.pruneWorktrees")}
						</Button>
						<Tooltip label={t("common.refresh")}>
							<ActionIcon variant="subtle" onClick={reload} aria-label={t("common.refresh")}>
								<ArrowClockwiseIcon size={16} />
							</ActionIcon>
						</Tooltip>
					</Group>
				</Group>

				{loading ? (
					<Group justify="center" py="md">
						<Loader size="sm" />
					</Group>
				) : (
					<ScrollArea.Autosize mah={400}>
						<Stack gap="md">
							{repositoryBranches.map(({ repository, resolvedPath, branches }) => {
								const deletableBranches = branches.filter(isDeletable);
								const allSelected =
									deletableBranches.length > 0 &&
									deletableBranches.every((branch) =>
										selectedKeys.has(branchKey(resolvedPath, branch.name)),
									);
								return (
									<Box key={repository.id}>
										<Group justify="space-between" align="center" mb="xs">
											<Text size="sm" fw={500}>
												{repository.name}
											</Text>
											{deletableBranches.length > 0 && (
												<Button
													variant="subtle"
													size="compact-xs"
													onClick={() => selectAllInRepo(resolvedPath, branches)}
												>
													{allSelected ? t("branches.deselectAll") : t("branches.selectAll")}
												</Button>
											)}
										</Group>
										{branches.length === 0 ? (
											<Text size="xs" c="dimmed">
												{t("branches.noBranches")}
											</Text>
										) : (
											<Stack gap={4}>
												{branches.map((branch) => {
													const key = branchKey(resolvedPath, branch.name);
													return (
														<BranchRow
															key={key}
															branch={branch}
															checked={selectedKeys.has(key)}
															onToggle={() => toggle(key)}
														/>
													);
												})}
											</Stack>
										)}
									</Box>
								);
							})}
						</Stack>
					</ScrollArea.Autosize>
				)}

				<Group justify="flex-end">
					<Button
						color="red"
						size="xs"
						loading={deleting}
						disabled={validSelectedKeys.length === 0}
						onClick={handleDeleteSelected}
					>
						{t("branches.deleteSelected", { count: validSelectedKeys.length })}
					</Button>
				</Group>

				<Divider />

				<Box>
					<Text fw={500}>{t("worktrees.title")}</Text>
					<Text size="sm" c="dimmed">
						{t("worktrees.description")}
					</Text>
				</Box>

				{loadingWorktrees ? (
					<Group justify="center" py="md">
						<Loader size="sm" />
					</Group>
				) : (
					<ScrollArea.Autosize mah={300}>
						<Stack gap="md">
							{repositoryWorktrees.map(({ repository, resolvedPath, worktrees }) => {
								const removableWorktrees = worktrees.filter((worktree) => !worktree.isDefault);
								return (
									<Box key={repository.id}>
										<Text size="sm" fw={500} mb="xs">
											{repository.name}
										</Text>
										{removableWorktrees.length === 0 ? (
											<Text size="xs" c="dimmed">
												{t("worktrees.none")}
											</Text>
										) : (
											<Stack gap={4}>
												{removableWorktrees.map((worktree) => {
													const key = worktreeKey(resolvedPath, worktree.path);
													return (
														<WorktreeRow
															key={key}
															worktree={worktree}
															checked={selectedWorktreeKeys.has(key)}
															onToggle={() => toggleWorktree(key)}
														/>
													);
												})}
											</Stack>
										)}
									</Box>
								);
							})}
						</Stack>
					</ScrollArea.Autosize>
				)}

				<Group justify="flex-end">
					<Button
						color="red"
						size="xs"
						loading={deletingWorktrees}
						disabled={validSelectedWorktreeKeys.length === 0}
						onClick={handleDeleteSelectedWorktrees}
					>
						{t("worktrees.deleteSelected", { count: validSelectedWorktreeKeys.length })}
					</Button>
				</Group>

				<Divider />

				<Group justify="flex-start">
					<Button variant="subtle" onClick={onClose}>
						{t("common.close")}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}

type WorktreeRowProps = {
	worktree: Worktree;
	checked: boolean;
	onToggle: () => void;
};

function WorktreeRow({ worktree, checked, onToggle }: WorktreeRowProps) {
	const { t } = useTranslation();

	return (
		<Group
			wrap="nowrap"
			gap="xs"
			px="xs"
			py={4}
			style={{ borderRadius: "var(--mantine-radius-sm)" }}
		>
			<Checkbox
				size="xs"
				checked={checked}
				onChange={onToggle}
				aria-label={t("worktrees.selectWorktree", { name: worktree.name })}
			/>
			<Box style={{ flex: 1, minWidth: 0 }}>
				<Text size="sm" truncate>
					{worktree.name}
				</Text>
				<Text size="xs" c="dimmed" truncate>
					{worktree.path}
				</Text>
			</Box>
			{worktree.branch && (
				<Badge size="xs" variant="light" color="blue">
					{worktree.branch}
				</Badge>
			)}
		</Group>
	);
}

type BranchRowProps = {
	branch: LocalBranch;
	checked: boolean;
	onToggle: () => void;
};

function BranchRow({ branch, checked, onToggle }: BranchRowProps) {
	const { t } = useTranslation();
	const deletable = isDeletable(branch);
	const dead = isDead(branch);

	const disabledReason = branch.isCurrent
		? t("branches.cannotDeleteCurrent")
		: branch.isCheckedOut
			? t("branches.cannotDeleteCheckedOut")
			: null;

	const row = (
		<Group
			wrap="nowrap"
			gap="xs"
			px="xs"
			py={4}
			style={{
				borderRadius: "var(--mantine-radius-sm)",
				backgroundColor: dead ? "var(--mantine-color-red-light)" : undefined,
			}}
		>
			<Checkbox
				size="xs"
				checked={checked}
				disabled={!deletable}
				onChange={onToggle}
				aria-label={t("branches.selectBranch", { branch: branch.name })}
			/>
			<Text size="sm" truncate style={{ flex: 1, minWidth: 0 }}>
				{branch.name}
			</Text>
			{branch.isCurrent && (
				<Badge size="xs" variant="light" color="blue">
					{t("branches.badgeCurrent")}
				</Badge>
			)}
			{branch.isCheckedOut && !branch.isCurrent && (
				<Badge size="xs" variant="light" color="gray">
					{t("branches.badgeCheckedOut")}
				</Badge>
			)}
			{branch.upstreamGone && (
				<Badge size="xs" variant="light" color="red">
					{t("branches.badgeGone")}
				</Badge>
			)}
			{branch.isMerged && !branch.upstreamGone && (
				<Badge size="xs" variant="light" color="teal">
					{t("branches.badgeMerged")}
				</Badge>
			)}
		</Group>
	);

	if (disabledReason) {
		return <Tooltip label={disabledReason}>{row}</Tooltip>;
	}

	return row;
}
