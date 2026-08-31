import { ActionIcon, Button, Card, Flex, Group, Modal, Stack, Text, Tooltip } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import {
	AppWindowIcon,
	GitBranchIcon,
	InfoIcon,
	LockIcon,
	NetworkIcon,
	TrashIcon,
} from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useActiveWorkspaces } from "@/hooks/useActiveWorkspaces";
import { useContextDiffStats } from "@/hooks/useContextDiffStats";
import type { CreateContextParams } from "@/hooks/useContexts";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Context, Project } from "@/models/Project";
import { CreateFromContextModal } from "./CreateFromContextModal";

type ContextCardProps = {
	context: Context;
	project: Project;
	allContexts: Context[];
	onDelete: (contextId: string) => Promise<void> | void;
	onCreate: (params: CreateContextParams) => Promise<void>;
};

export function ContextCard({ context, project, allContexts, onDelete, onCreate }: ContextCardProps) {
	const { t } = useTranslation();
	const { open, isAvailable } = useOpenInIde();
	const { isActive } = useActiveWorkspaces();
	const { statsByRepository, hasBaseContext } = useContextDiffStats(context, project, allContexts);
	const [opened, { open: openModal, close: closeModal }] = useDisclosure(false);
	const [createOpened, { open: openCreateModal, close: closeCreateModal }] = useDisclosure(false);
	const [deleting, setDeleting] = useState(false);

	const branchName = context.branches[0]?.branch ?? context.name;
	const projectBase = project.path.endsWith("/") ? project.path.slice(0, -1) : project.path;
	const worktreePath = `${projectBase}/.worktrees/${context.name}`;
	const isContextActive = isActive(worktreePath);

	async function handleOpenInIDE() {
		const worktreeExists = await invoke<boolean>("check_path_exists", { path: worktreePath });
		await open(worktreeExists ? worktreePath : project.path);
	}

	async function handleDelete() {
		setDeleting(true);
		await onDelete(context.id);
		setDeleting(false);
		closeModal();
	}

	return (
		<>
			<Card
				withBorder
				padding="sm"
				style={isContextActive ? { borderColor: "var(--mantine-color-green-6)" } : undefined}
			>
				<Stack gap="xs">
					<Flex justify="space-between">
						<Flex gap="xs">
							<NetworkIcon
								size={16}
								color={isContextActive ? "var(--mantine-color-green-6)" : undefined}
							/>
							<Text fw={600} size="xs" c={context.isDefault ? "primary.2" : undefined}>
								{context.isDefault ? t("common.default") : context.name}
							</Text>
						</Flex>
						<Flex gap={3}>
							{context.baseContextName && (
								<Tooltip label={`${t("contexts.baseContext")}: ${context.baseContextName}`}>
									<ActionIcon
										variant="subtle"
										size="sm"
										radius={2}
										bg="transparent"
										disabled
										aria-label="Base context info"
									>
										<InfoIcon size={16} />
									</ActionIcon>
								</Tooltip>
							)}
							<Tooltip label={t("contexts.createFromContext")}>
								<ActionIcon
									variant="subtle"
									size="sm"
									radius={2}
									onClick={openCreateModal}
									aria-label="Create context from this"
								>
									<GitBranchIcon size={16} />
								</ActionIcon>
							</Tooltip>
							<Tooltip label={t("contexts.openAllInIde")}>
								<ActionIcon
									variant="subtle"
									radius={2}
									size="sm"
									onClick={handleOpenInIDE}
									disabled={!isAvailable}
									aria-label="Open all in IDE"
								>
									<AppWindowIcon size={16} />
								</ActionIcon>
							</Tooltip>
							{!context.isDefault && (
								<Tooltip label={t("contexts.deleteContext")}>
									<ActionIcon
										variant="subtle"
										color="red"
										radius={2}
										size="sm"
										onClick={openModal}
										aria-label="Delete context"
									>
										<TrashIcon size={16} />
									</ActionIcon>
								</Tooltip>
							)}
						</Flex>
					</Flex>
					<Stack gap={0}>
						{context.branches.map((cb, index) => {
							const repo = project.findRepository(cb.repositoryId);
							const stats = statsByRepository[cb.repositoryId];
							const isLast = index === context.branches.length - 1;
							const hasChanges = stats && (stats.filesAdded > 0 || stats.filesModified > 0 || stats.filesDeleted > 0 || stats.insertions > 0 || stats.deletions > 0);
							return (
								<Flex
									key={cb.repositoryId}
									align="center"
									justify="space-between"
								>
									<Text
										size="xs"
										c="dark.1"
										style={{ fontFamily: "monospace" }}
									>
										{isLast ? "└─ " : "├─ "}
										{repo?.name ?? "?"} (
										{cb.linked ? (
											<LockIcon size={10} />
										) : (
											<GitBranchIcon color="var(--mantine-color-primary-1)" size={10} />
										)}
										)
									</Text>
									{!context.isDefault && !hasBaseContext && (
										<Text size="xs" c="red.3" style={{ fontFamily: "monospace" }}>
											whoops
										</Text>
									)}
									{hasChanges && (
										<Text size="xs" style={{ fontFamily: "monospace" }}>
											{stats.filesAdded > 0 && (
												<Text span size="xs" c="green.5">+{stats.filesAdded.toLocaleString()}</Text>
											)}
											{stats.filesModified > 0 && (
												<Text span size="xs" c="yellow.5"> ~{stats.filesModified.toLocaleString()}</Text>
											)}
											{stats.filesDeleted > 0 && (
												<Text span size="xs" c="red.5"> -{stats.filesDeleted.toLocaleString()}</Text>
											)}
											<Text span size="xs" c="dimmed"> files </Text>
											<Text span size="xs" c="green.5">
												+{stats.insertions.toLocaleString()}
											</Text>
											<Text span size="xs" c="dimmed">/</Text>
											<Text span size="xs" c="red.5">
												-{stats.deletions.toLocaleString()}
											</Text>
											<Text span size="xs" c="dimmed"> lines</Text>
										</Text>
									)}
								</Flex>
							);
						})}
					</Stack>
				</Stack>
			</Card>

			<Modal opened={opened} onClose={closeModal} title={t("contexts.deleteConfirmTitle")}>
				<Stack gap="md">
					<Text size="sm">{t("contexts.deleteConfirmMessage", { name: context.name })}</Text>
					<Text size="sm" c="dimmed">
						{t("contexts.branch")}: {branchName}
					</Text>
					<Text size="sm" c="red">
						{t("contexts.deleteWarning")}
					</Text>
					<Group justify="flex-end">
						<Button variant="subtle" onClick={closeModal}>
							{t("common.cancel")}
						</Button>
						<Button color="red" onClick={handleDelete} loading={deleting}>
							{t("common.delete")}
						</Button>
					</Group>
				</Stack>
			</Modal>

			<CreateFromContextModal
				opened={createOpened}
				onClose={closeCreateModal}
				sourceContext={context}
				project={project}
				onCreate={onCreate}
			/>
		</>
	);
}
