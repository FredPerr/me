import { ActionIcon, Badge, Card, Group, Stack, Text, Tooltip } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { GitRemoteLink } from "@/components/shared/GitRemoteLink";
import { getIconComponent } from "@/components/shared/IconPicker";
import { useOpenInIde } from "@/hooks/useOpenInIde";
import type { SubProject } from "@/models/Project";
import { resolvePath } from "@/utils/resolvePath";

type SubProjectEntryProps = {
	subproject: SubProject;
	projectPath: string;
	isActive: (path: string) => boolean;
};

export function SubProjectEntry({ subproject, projectPath, isActive }: SubProjectEntryProps) {
	const { open, isAvailable } = useOpenInIde();
	const [resolvedPath, setResolvedPath] = useState<string | undefined>();
	const Icon = subproject.icon ? getIconComponent(subproject.icon) : null;
	const active = resolvedPath ? isActive(resolvedPath) : false;

	useEffect(() => {
		resolvePath(subproject.relPath, { basePath: projectPath }).then(setResolvedPath);
	}, [subproject.relPath, projectPath]);

	async function handleOpen() {
		if (resolvedPath) {
			await open(resolvedPath);
		}
	}

	return (
		<Card withBorder padding="sm" style={active ? { borderColor: "var(--mantine-color-green-6)" } : undefined}>
			<Stack gap="xs">
				<Group justify="space-between">
					<Group gap="xs">
						{Icon && <Icon size={20} />}
						<div>
							<Text fw={700} size="md">
								{subproject.name}
							</Text>
							<Text size="xs" c="dimmed" ff="monospace">
								{subproject.relPath}
							</Text>
						</div>
					</Group>
					<Group gap="xs">
						{active && <Badge size="xs" color="green">active</Badge>}
						<GitRemoteLink path={resolvedPath} />
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
				</Group>
			</Stack>
		</Card>
	);
}
