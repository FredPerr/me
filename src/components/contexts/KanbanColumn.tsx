import { useDroppable } from "@dnd-kit/core";
import { Badge, Group, Paper, Stack, Text } from "@mantine/core";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { CONTEXT_STATUS_COLORS, type ContextStatus } from "@/models/ContextStatus";

type KanbanColumnProps = {
	status: ContextStatus;
	count: number;
	children: ReactNode;
};

export function KanbanColumn({ status, count, children }: KanbanColumnProps) {
	const { t } = useTranslation();
	const { setNodeRef, isOver } = useDroppable({ id: status });
	const color = CONTEXT_STATUS_COLORS[status];

	return (
		<Stack gap="xs" style={{ minWidth: 340, flex: "1 0 340px", minHeight: 0 }}>
			<Group gap="xs" px={4}>
				<Text size="xs" fw={700} tt="uppercase" c={`${color}.4`}>
					{t(`contexts.kanban.status.${status}`)}
				</Text>
				<Badge size="sm" variant="light" color={color}>
					{count}
				</Badge>
			</Group>
			<Paper
				ref={setNodeRef}
				withBorder
				radius="md"
				p={8}
				style={{
					flex: 1,
					minHeight: 120,
					overflowY: "auto",
					borderTop: `2px solid var(--mantine-color-${color}-6)`,
					backgroundColor: isOver
						? `var(--mantine-color-${color}-light)`
						: "var(--mantine-color-dark-7)",
					transition: "background-color 120ms ease",
				}}
			>
				<Stack gap={8}>
					{count === 0 ? (
						<Text size="xs" c="dimmed" ta="center" py="md">
							{t("contexts.kanban.columnEmpty")}
						</Text>
					) : (
						children
					)}
				</Stack>
			</Paper>
		</Stack>
	);
}
