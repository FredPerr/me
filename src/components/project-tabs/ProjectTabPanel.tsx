import { ActionIcon, Badge, Button, Divider, Group, SimpleGrid, Stack, TabsPanel, Tooltip } from "@mantine/core";
import { ArrowSquareOutIcon, ArrowsClockwiseIcon } from "@phosphor-icons/react";
import { GitRemoteLink } from "@/components/shared/GitRemoteLink";
import { useActiveWorkspaces } from "@/hooks/useActiveWorkspaces";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Project } from "@/models/Project";
import { SubProjectEntry } from "./SubProjectEntry";

type ProjectTabPanelProps = {
	project: Project;
};

export function ProjectTabPanel({ project }: ProjectTabPanelProps) {
	const { open, isAvailable } = useOpenInIde();
	const { isActive, refresh } = useActiveWorkspaces();

	return (
		<TabsPanel value={project.tag}>
			<Stack gap="sm" py="md">
				<Group justify="space-between">
					<Group gap="xs">
						{isActive(project.path) && <Badge size="xs" color="green">active</Badge>}
					</Group>
					<Group gap="xs">
						<Tooltip label="Refresh workspaces">
							<ActionIcon variant="subtle" size="sm" onClick={refresh} aria-label="Refresh workspaces">
								<ArrowsClockwiseIcon size={16} />
							</ActionIcon>
						</Tooltip>
						<GitRemoteLink path={project.path} />
						<Button
							leftSection={<ArrowSquareOutIcon size={16} />}
							size="xs"
							onClick={() => open(project.path)}
							disabled={!project.path || !isAvailable}
						>
							Open in IDE
						</Button>
					</Group>
				</Group>
				{project.subprojects.length > 0 && (
					<>
						<Divider />
						<SimpleGrid cols={{ base: 1, sm: 2, md: 3 }} spacing="sm">
							{project.subprojects.map((subproject) => (
								<SubProjectEntry
									key={subproject.id}
									subproject={subproject}
									projectPath={project.path}
									isActive={isActive}
								/>
							))}
						</SimpleGrid>
					</>
				)}
			</Stack>
		</TabsPanel>
	);
}
