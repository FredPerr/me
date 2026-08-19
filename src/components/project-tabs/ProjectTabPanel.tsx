import { Button, Divider, Group, SimpleGrid, Stack, TabsPanel } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { Project } from "@/models/Project";
import { SubProjectEntry } from "./SubProjectEntry";

type ProjectTabPanelProps = {
	project: Project;
};

export function ProjectTabPanel({ project }: ProjectTabPanelProps) {
	const { open, isAvailable } = useOpenInIde();

	return (
		<TabsPanel value={project.tag}>
			<Stack gap="sm" py="md">
				<Group justify="flex-end">
					<Button
						leftSection={<ArrowSquareOutIcon size={16} />}
						size="xs"
						onClick={() => open(project.path)}
						disabled={!project.path || !isAvailable}
					>
						Open in IDE
					</Button>
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
								/>
							))}
						</SimpleGrid>
					</>
				)}
			</Stack>
		</TabsPanel>
	);
}
