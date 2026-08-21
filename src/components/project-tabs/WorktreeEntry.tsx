import { ActionIcon, Badge, Card, Group, Text, Tooltip } from "@mantine/core";
import { ArrowSquareOutIcon, FileIcon, GitBranchIcon, PlusMinusIcon } from "@phosphor-icons/react";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import { useWorktreeStatus } from "@/hooks/useWorktreeStatus";
import type { Worktree } from "@/hooks/useWorktrees";
import { PullRequestButton } from "./PullRequestButton";

type WorktreeEntryProps = {
	worktree: Worktree;
	isActive: (path: string) => boolean;
	remoteUrl: string | null;
};

export function WorktreeEntry({ worktree, isActive, remoteUrl }: WorktreeEntryProps) {
	const { open, isAvailable } = useOpenInIde();
	const { status } = useWorktreeStatus(worktree.path);
	const active = isActive(worktree.path);

	return (
		<Card withBorder padding="sm" style={active ? { borderColor: "var(--mantine-color-green-6)" } : undefined}>
			<Group justify="space-between">
				<Group gap="xs">
					<GitBranchIcon size={16} />
					<div>
						<Text fw={500} size="sm">
							{worktree.branch ?? worktree.name}
						</Text>
						<Text size="xs" c="dimmed" ff="monospace">
							{worktree.path}
						</Text>
					</div>
				</Group>
				<Group gap="xs">
					{active && <Badge size="xs" color="green">active</Badge>}
					{worktree.is_default && <Badge size="xs" variant="outline">default</Badge>}
					{status && status.changed_files > 0 && (
						<Tooltip label={`${status.changed_files} file(s) changed`}>
							<Group gap={2}>
								<FileIcon size={14} />
								<Text size="xs" c="dimmed">{status.changed_files}</Text>
							</Group>
						</Tooltip>
					)}
					{status && (status.insertions > 0 || status.deletions > 0) && (
						<Tooltip label={`+${status.insertions} / -${status.deletions} lines`}>
							<Group gap={2}>
								<PlusMinusIcon size={14} />
								<Text size="xs" c="green.5">+{status.insertions}</Text>
								<Text size="xs" c="red.5">-{status.deletions}</Text>
							</Group>
						</Tooltip>
					)}
					{!worktree.is_default && (
						<PullRequestButton
							prUrl={status?.pr_url ?? null}
							remoteUrl={remoteUrl ?? undefined}
						/>
					)}
					<Tooltip label="Open in IDE">
						<ActionIcon
							variant="subtle"
							size="sm"
							onClick={() => open(worktree.path)}
							disabled={!isAvailable}
							aria-label={`Open ${worktree.name} in IDE`}
						>
							<ArrowSquareOutIcon size={16} />
						</ActionIcon>
					</Tooltip>
				</Group>
			</Group>
		</Card>
	);
}
