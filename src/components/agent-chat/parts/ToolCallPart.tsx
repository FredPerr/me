import { Badge, Group, Loader, Paper, Text } from "@mantine/core";
import { CheckCircleIcon, WrenchIcon, XCircleIcon } from "@phosphor-icons/react";
import type { ReactNode } from "react";
import type { ToolCallPart as ToolCallPartModel } from "@/models/agent-chat/AgentMessage";

type ToolCallPartProps = {
	part: ToolCallPartModel;
};

function statusIcon(status: ToolCallPartModel["status"]): ReactNode {
	if (status === "running") return <Loader size={14} />;
	if (status === "completed")
		return <CheckCircleIcon size={16} color="var(--mantine-color-teal-4)" />;
	return <XCircleIcon size={16} color="var(--mantine-color-red-4)" />;
}

/**
 * Renders a tool/command invocation the agent made, with its name, an optional
 * argument summary, and a status indicator. Shown when structured tool events
 * are available.
 */
export function ToolCallPart({ part }: ToolCallPartProps) {
	return (
		<Paper withBorder radius="sm" px="sm" py={6} bg="dark.6">
			<Group gap="xs" wrap="nowrap">
				<WrenchIcon size={14} color="var(--mantine-color-dimmed)" />
				<Text size="sm" ff="monospace" fw={600}>
					{part.name}
				</Text>
				{part.summary && (
					<Text size="xs" c="dimmed" style={{ flex: 1, minWidth: 0 }} truncate>
						{part.summary}
					</Text>
				)}
				<Badge size="xs" variant="transparent" leftSection={statusIcon(part.status)}>
					{part.status}
				</Badge>
			</Group>
		</Paper>
	);
}
