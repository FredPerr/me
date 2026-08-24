import { ActionIcon, Badge, Button, Card, Group, Modal, Stack, Text, Tooltip } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import {
	ArrowSquareOutIcon,
	GitBranchIcon,
	GlobeIcon,
	NetworkIcon,
	TrashIcon,
} from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useActiveWorkspaces } from "@/hooks/useActiveWorkspaces";
import type { CreateContextParams } from "@/hooks/useContexts";
import type { MatchedPort } from "@/hooks/useListeningPorts";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Context, Project } from "@/models/Project";
import { CreateFromContextModal } from "./CreateFromContextModal";

type ContextCardProps = {
	context: Context;
	project: Project;
	onDelete: (contextId: string) => void;
	onCreate: (params: CreateContextParams) => Promise<void>;
	ports: MatchedPort[];
};

export function ContextCard({ context, project, onDelete, onCreate, ports }: ContextCardProps) {
	const { t } = useTranslation();
	const { open, isAvailable } = useOpenInIde();
	const { isActive } = useActiveWorkspaces();
	const [opened, { open: openModal, close: closeModal }] = useDisclosure(false);
	const [createOpened, { open: openCreateModal, close: closeCreateModal }] = useDisclosure(false);
	const [deleting, setDeleting] = useState(false);

	const branchName = context.branches[0]?.branch ?? context.name;
	const projectBase = project.path.endsWith("/") ? project.path.slice(0, -1) : project.path;
	const worktreePath = `${projectBase}/.worktrees/${context.name}`;
	const isContextActive = isActive(worktreePath);

	async function handleOpenAll() {
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
					<Group justify="space-between">
						<Group gap="xs">
							<NetworkIcon size={16} />
							<Text fw={500} size="sm">
								{context.name}
							</Text>
							{context.isDefault && (
								<Badge size="xs" variant="outline">
									{t("common.default")}
								</Badge>
							)}
							{isContextActive && (
								<Badge size="xs" color="green">
									{t("common.active")}
								</Badge>
							)}
						</Group>
						<Group gap="xs">
							<Tooltip label={t("contexts.createFromContext")}>
								<ActionIcon
									variant="subtle"
									size="sm"
									onClick={openCreateModal}
									aria-label="Create context from this"
								>
									<GitBranchIcon size={16} />
								</ActionIcon>
							</Tooltip>
							<Tooltip label={t("contexts.openAllInIde")}>
								<ActionIcon
									variant="subtle"
									size="sm"
									onClick={handleOpenAll}
									disabled={!isAvailable}
									aria-label="Open all in IDE"
								>
									<ArrowSquareOutIcon size={16} />
								</ActionIcon>
							</Tooltip>
							{!context.isDefault && (
								<Tooltip label={t("contexts.deleteContext")}>
									<ActionIcon
										variant="subtle"
										color="red"
										size="sm"
										onClick={openModal}
										aria-label="Delete context"
									>
										<TrashIcon size={16} />
									</ActionIcon>
								</Tooltip>
							)}
						</Group>
					</Group>
					<Group gap="xs">
						{context.branches.map((cb) => {
							const repo = project.findRepository(cb.repositoryId);
							return (
								<Badge
									key={cb.repositoryId}
									size="xs"
									variant={cb.linked ? "outline" : "light"}
									color={cb.linked ? "gray" : undefined}
								>
									{repo?.name ?? "?"}: {cb.branch}
									{cb.linked ? ` (${t("contexts.linked")})` : ""}
								</Badge>
							);
						})}
					</Group>
					{ports.length > 0 && (
						<Group gap="xs">
							{ports.map((p) => (
								<Tooltip
									key={p.port}
									label={`${p.processName}${p.repositoryName ? ` — ${p.repositoryName}` : ""}`}
								>
									<Badge
										size="xs"
										variant="dot"
										color="green"
										style={{ cursor: "pointer" }}
										onClick={() => openUrl(`http://localhost:${p.port}`)}
									>
										<Group gap={4} wrap="nowrap">
											<GlobeIcon size={10} />:{p.port}
										</Group>
									</Badge>
								</Tooltip>
							))}
						</Group>
					)}
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
