import {
	ActionIcon,
	Button,
	Card,
	Flex,
	Group,
	Loader,
	Modal,
	SimpleGrid,
	Stack,
	Text,
	Tooltip,
} from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import {
	AppWindowIcon,
	CubeIcon,
	GitBranchIcon,
	GitPullRequestIcon,
	InfoIcon,
	LockIcon,
	RobotIcon,
	TrashIcon,
} from "@phosphor-icons/react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { ContextView, ProjectView } from "@/bindings";
import { useActiveWorkspaces } from "@/hooks/useActiveWorkspaces";
import { useContextDiffStats } from "@/hooks/useContextDiffStats";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import { useDeleteContext, useSetContextStatic } from "@/hooks/workspace/useContextMutations";
import type { PullRequestLookup } from "@/models/git-hosting/branchPullRequests";
import { findBaseContext, findEffectiveRepository } from "@/models/projectView";
import "./ContextCard.css";
import { CreateFromContextModal } from "./CreateFromContextModal";
import { CreatePullRequestModal } from "./CreatePullRequestModal";
import { PullRequestStatusIcon } from "./PullRequestStatusIcon";

type ContextCardProps = {
	context: ContextView;
	project: ProjectView;
	/** Looks up the pull request status for a sub-repo branch, when connected. */
	pullRequestLookup?: PullRequestLookup;
	/** Opens the shared Kiro conversation modal for this context. */
	onOpenKiro?: (context: ContextView) => void;
	isKiroRunning?: boolean;
	hasKiroHistory?: boolean;
	dragHandle?: ReactNode;
};

export function ContextCard({
	context,
	project,
	pullRequestLookup,
	onOpenKiro,
	isKiroRunning = false,
	hasKiroHistory = false,
	dragHandle,
}: ContextCardProps) {
	const { t } = useTranslation();
	const { open: openInIde, isAvailable } = useOpenInIde();
	const { isActive } = useActiveWorkspaces();
	const deleteContext = useDeleteContext(project.tag);
	const { mutate: setContextStatic } = useSetContextStatic(project.tag);
	const { statsByRepository, hasBaseContext } = useContextDiffStats(context, project);
	const [opened, { open: openModal, close: closeModal }] = useDisclosure(false);
	const [createOpened, { open: openCreateModal, close: closeCreateModal }] = useDisclosure(false);
	const [prOpened, { open: openPrModal, close: closePrModal }] = useDisclosure(false);

	const branchName = context.branches[0]?.branch ?? context.name;
	const isContextActive = isActive(context.folderPath);
	const baseContext = findBaseContext(project, context);
	const canDraftPullRequests = context.status === "needsHuman" || context.status === "review";
	const canRunKiro = Boolean(onOpenKiro) && !context.isDefault;

	async function handleOpenInIDE() {
		await openInIde(context.idePath);
	}

	function handleOpenKiro() {
		onOpenKiro?.(context);
	}

	function handleDraftPullRequests() {
		openPrModal();
	}

	function handleDelete() {
		deleteContext.mutate(context.id, { onSuccess: closeModal });
	}

	return (
		<>
			<Card
				withBorder
				padding="xs"
				style={isContextActive ? { borderColor: "var(--mantine-color-green-6)" } : undefined}
			>
				<Flex gap="xs" align="flex-start" wrap="nowrap">
					<Stack gap="xs" style={{ flex: 1, minWidth: 0 }}>
						<Flex gap="xs" align="center">
							{dragHandle}
							<Text
								fw={600}
								size="xs"
								style={{ minWidth: 0, wordBreak: "break-word" }}
								c={context.isDefault ? "primary.2" : isContextActive ? "green.6" : undefined}
							>
								{context.isDefault ? t("common.default") : context.name}
							</Text>
						</Flex>
						<Stack gap="xs">
							{baseContext && (
								<Flex align="center" gap={6}>
									<InfoIcon size={12} />
									<Text size="xs" c="dimmed">
										{t("contexts.baseContext")}:{" "}
										{baseContext.isDefault ? t("common.default") : baseContext.name}
									</Text>
								</Flex>
							)}
							<Stack gap={0}>
								{context.branches.map((cb, index) => {
									const repo = findEffectiveRepository(project, cb.repositoryId);
									const stats = statsByRepository[cb.repositoryId];
									const isLast = index === context.branches.length - 1;
									const hasChanges =
										stats &&
										(stats.filesAdded > 0 ||
											stats.filesModified > 0 ||
											stats.filesDeleted > 0 ||
											stats.insertions > 0 ||
											stats.deletions > 0);
									const pullRequest = pullRequestLookup?.(cb.repositoryId, cb.branch) ?? null;
									return (
										<Flex key={cb.repositoryId} align="center" justify="space-between">
											<Flex align="center" gap={2}>
												<Text size="xs" c="dark.1" style={{ fontFamily: "monospace" }}>
													{isLast ? "└─ " : "├─ "}
													{repo?.name ?? "?"} (
													{cb.checkout === "linked" ? (
														<LockIcon size={10} />
													) : (
														<GitBranchIcon color="var(--mantine-color-primary-1)" size={10} />
													)}
													)
												</Text>
												{pullRequest && <PullRequestStatusIcon pullRequest={pullRequest} />}
											</Flex>
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
						</Stack>
					</Stack>

					<SimpleGrid cols={2} spacing={3} verticalSpacing={3} style={{ flexShrink: 0 }}>
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
						{!context.isDefault && (
							<Tooltip
								label={context.isStatic ? t("contexts.static.unmark") : t("contexts.static.mark")}
							>
								<ActionIcon
									variant={context.isStatic ? "light" : "subtle"}
									color={context.isStatic ? "teal" : "gray"}
									size="sm"
									radius={2}
									onClick={() =>
										setContextStatic({ contextId: context.id, isStatic: !context.isStatic })
									}
									aria-label={
										context.isStatic ? t("contexts.static.unmark") : t("contexts.static.mark")
									}
								>
									<CubeIcon size={16} />
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
					</SimpleGrid>
				</Flex>
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
						<Button color="red" onClick={handleDelete} loading={deleteContext.isPending}>
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
			/>

			<CreatePullRequestModal
				opened={prOpened}
				onClose={closePrModal}
				project={project}
				context={context}
			/>
		</>
	);
}
