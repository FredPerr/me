import { Stack, Text, Title } from "@mantine/core";
import type { Worktree } from "@/hooks/useWorktrees";
import { WorktreeEntry } from "./WorktreeEntry";

type WorktreeListProps = {
	worktrees: Worktree[];
	isActive: (path: string) => boolean;
	remoteUrl: string | null;
};

export function WorktreeList({ worktrees, isActive, remoteUrl }: WorktreeListProps) {
	return (
		<Stack gap="xs">
			<Title order={5} c="dimmed">
				Worktrees
			</Title>
			{worktrees.length === 0 && <Text size="sm" c="dimmed">No worktrees found.</Text>}
			{worktrees.map((worktree) => (
				<WorktreeEntry
					key={worktree.path}
					worktree={worktree}
					isActive={isActive}
					remoteUrl={remoteUrl}
				/>
			))}
		</Stack>
	);
}
