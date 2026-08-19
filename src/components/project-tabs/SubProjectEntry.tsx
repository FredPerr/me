import { ActionIcon, Card, Group, Text, Tooltip } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { SubProject } from "@/models/Project";
import { resolvePath } from "@/utils/resolvePath";

type SubProjectEntryProps = {
	subproject: SubProject;
	projectPath: string;
};

export function SubProjectEntry({ subproject, projectPath }: SubProjectEntryProps) {
	const { open, isAvailable } = useOpenInIde();

	async function handleOpen() {
		const resolvedPath = await resolvePath(subproject.relPath, {
			basePath: projectPath,
		});
		await open(resolvedPath);
	}

	return (
		<Card withBorder padding="sm">
			<Group justify="space-between">
				<div>
					<Text fw={700} size="md">
						{subproject.name}
					</Text>
					<Text size="xs" c="dimmed" ff="monospace">
						{subproject.relPath}
					</Text>
				</div>
				<Tooltip label="Open in IDE">
					<ActionIcon
						variant="subtle"
						size="sm"
						onClick={handleOpen}
						disabled={!isAvailable}
						aria-label={`Open ${subproject.name} in IDE`}
					>
						<ArrowSquareOutIcon size={16} />
					</ActionIcon>
				</Tooltip>
			</Group>
		</Card>
	);
}
