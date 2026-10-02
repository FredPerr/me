import {
	ActionIcon,
	Button,
	Card,
	Collapse,
	Flex,
	Group,
	Loader,
	Modal,
	Stack,
	Text,
	Tooltip,
} from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import {
	AppWindowIcon,
	ArrowsInIcon,
	ArrowsOutIcon,
	GitBranchIcon,
	GitPullRequestIcon,
	InfoIcon,
	LockIcon,
	RobotIcon,
	TrashIcon,
} from "@phosphor-icons/react";
import type { ReactNode } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useActiveWorkspaces } from "@/hooks/useActiveWorkspaces";
import { useContextDiffStats } from "@/hooks/useContextDiffStats";
import { type CreateContextParams, useContexts } from "@/hooks/useContexts";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Context, Project } from "@/models/Project";
import "./ContextCard.css";
import { CreateFromContextModal } from "./CreateFromContextModal";
import { CreatePullRequestModal } from "./CreatePullRequestModal";

type ContextCardProps = {
	context: Context;
	project: Project;
	allContexts: Context[];
	onDelete: (contextId: string) => Promise<void> | void;
	onCreate: (params: CreateContextParams) => Promise<void>;
	/** Persists the expanded/collapsed state for this context. */
	onExpandedChange?: (contextId: string, expanded: boolean) => void;
	/** Opens the shared Kiro conversation modal for this context. */
	onOpenKiro?: (context: Context) => void;
	isKiroRunning?: boolean;
	hasKiroHistory?: boolean;
	dragHandle?: ReactNode;
};

export function ContextCard({
	context,
	project,
	allContexts,
	onDelete,
	onCreate,
	onExpandedChange,
	onOpenKiro,
	isKiroRunning = false,
	hasKiroHistory = false,
	dragHandle,
}: ContextCardProps) {
	const { t } = useTranslation();
	const { openFirstExisting, isAvailable } = useOpenInIde();
	const { isActive } = useActiveWorkspaces();
	const { savePullRequestDrafts } = useContexts(project);
	const { statsByRepository, hasBaseContext } = useContextDiffStats(context, project, allContexts);
	const [opened, { open: openModal, close: closeModal }] = useDisclosure(false);
	const [createOpened, { open: openCreateModal, close: closeCreateModal }] = useDisclosure(false);
	const [prOpened, { open: openPrModal, close: closePrModal }] = useDisclosure(false);
	const [expanded, { toggle }] = useDisclosure(context.expanded);

	function toggleExpanded() {
		toggle();
		onExpandedChange?.(context.id, !expanded);
	}
	const [deleting, setDeleting] = useState(false);

	const branchName = context.branches[0]?.branch ?? context.name;
	const contextFolderPath = context.getContextFolderPath(project.path);
	const isContextActive = isActive(contextFolderPath);
	const canDraftPullRequests = context.status === "needsHuman" || context.status === "review";
	const canRunKiro = Boolean(onOpenKiro) && !context.isDefault;

	async function handleOpenInIDE() {
		await openFirstExisting(project.resolveContextIdePathCandidates(context));
	}

	function handleOpenKiro() {
		onOpenKiro?.(context);
	}

	function handleDraftPullRequests() {
		openPrModal();
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
				padding="xs"
				style={isContextActive ? { borderColor: "var(--mantine-color-green-6)" } : undefined}
			>
				<Stack gap="xs">
					<Flex justify="space-between">
						<Flex gap="xs" align="center">
							{dragHandle}
							<Text
								fw={600}
								size="xs"
								c={context.isDefault ? "primary.2" : isContextActive ? "green.6" : undefined}
							>
								{context.isDefault ? t("common.default") : context.name}
							</Text>
						</Flex>
						<Flex gap={3}>
							{canRunKiro && (
								<Tooltip
									label={
										isKiroRunning
											? t("contexts.kiro.running")
											: hasKiroHistory
												? t("contexts.kiro.openConversation")
												: t("contexts.kiro.run")
									}
								>
									<ActionIcon
										variant={isKiroRunning ? "light" : "subtle"}
										color={isKiroRunning ? "grape" : undefined}
										size="sm"
										radius={2}
										onClick={handleOpenKiro}
										aria-label={t("contexts.kiro.openConversation")}
									>
										{isKiroRunning ? <Loader size={14} color="grape" /> : <RobotIcon size={16} />}
									</ActionIcon>
								</Tooltip>
							)}
							{!context.isDefault && canDraftPullRequests && (
								<Tooltip label={t("contexts.draftPullRequest")}>
									<ActionIcon
										variant="subtle"
										size="sm"
										radius={2}
										onClick={handleDraftPullRequests}
										aria-label="Draft pull requests"
									>
										<GitPullRequestIcon size={16} />
									</ActionIcon>
								</Tooltip>
							)}
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
							<Tooltip label={expanded ? t("contexts.collapse") : t("contexts.expand")}>
								<ActionIcon
									variant="subtle"
									radius={2}
									size="sm"
									onClick={toggleExpanded}
									aria-expanded={expanded}
									aria-label={expanded ? t("contexts.collapse") : t("contexts.expand")}
								>
									{expanded ? <ArrowsInIcon size={16} /> : <ArrowsOutIcon size={16} />}
								</ActionIcon>
							</Tooltip>
						</Flex>
					</Flex>
					<Collapse expanded={expanded}>
						<Stack gap="xs">
							{context.baseContextName && (
								<Flex align="center" gap={6}>
									<InfoIcon size={12} />
									<Text size="xs" c="dimmed">
										{t("contexts.baseContext")}: {context.baseContextName}
									</Text>
								</Flex>
							)}
							<Stack gap={0}>
								{context.branches.map((cb, index) => {
									const repo = project.findEffectiveRepository(cb.repositoryId);
									const stats = statsByRepository[cb.repositoryId];
									const isLast = index === context.branches.length - 1;
									const hasChanges =
										stats &&
										(stats.filesAdded > 0 ||
											stats.filesModified > 0 ||
											stats.filesDeleted > 0 ||
											stats.insertions > 0 ||
											stats.deletions > 0);
									return (
										<Flex key={cb.repositoryId} align="center" justify="space-between">
											<Text size="xs" c="dark.1" style={{ fontFamily: "monospace" }}>
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
														<Text span size="xs" c="green.5">
															+{stats.filesAdded.toLocaleString()}
														</Text>
													)}
													{stats.filesModified > 0 && (
														<Text span size="xs" c="yellow.5">
															{" "}
															~{stats.filesModified.toLocaleString()}
														</Text>
													)}
													{stats.filesDeleted > 0 && (
														<Text span size="xs" c="red.5">
															{" "}
															-{stats.filesDeleted.toLocaleString()}
														</Text>
													)}
													<Text span size="xs" c="dimmed">
														{" "}
														files{" "}
													</Text>
													<Text span size="xs" c="green.5">
														+{stats.insertions.toLocaleString()}
													</Text>
													<Text span size="xs" c="dimmed">
														/
													</Text>
													<Text span size="xs" c="red.5">
														-{stats.deletions.toLocaleString()}
													</Text>
													<Text span size="xs" c="dimmed">
														{" "}
														lines
													</Text>
												</Text>
											)}
										</Flex>
									);
								})}
							</Stack>
							<Group gap={3} justify="flex-end">
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
								{!context.isDefault && (
									<Tooltip label={t("contexts.deleteContext")}>
										<ActionIcon
											variant="subtle"
											color="gray"
											size="sm"
											radius={2}
											onClick={openModal}
											aria-label="Delete context"
											className="context-card-delete"
										>
											<TrashIcon size={16} />
										</ActionIcon>
									</Tooltip>
								)}
							</Group>
						</Stack>
					</Collapse>
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

			<CreatePullRequestModal
				opened={prOpened}
				onClose={closePrModal}
				project={project}
				context={context}
				allContexts={allContexts}
				onSaveDrafts={savePullRequestDrafts}
			/>
		</>
	);
}
