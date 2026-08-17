import { Button, Group, Stack, TabsPanel, Text } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Project } from "@/models/Project";

type ProjectTabPanelProps = {
	project: Project;
};

export function ProjectTabPanel({ project }: ProjectTabPanelProps) {
	const { open, isAvailable } = useOpenInIde();

	return (
		<TabsPanel value={project.tag}>
			<Stack gap="sm" py="md">
				<Group justify="space-between">
					<Text>{project.name}</Text>
					<Button
						leftSection={<ArrowSquareOutIcon size={16} />}
						size="xs"
						onClick={() => open(project.path)}
						disabled={!project.path || !isAvailable}
					>
						Open in IDE
					</Button>
				</Group>
			</Stack>
		</TabsPanel>
	);
}
