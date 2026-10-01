import { useDroppable } from "@dnd-kit/core";
import { Badge, Group, Paper, Stack, Text } from "@mantine/core";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { ContextStatus } from "@/models/ContextStatus";

type KanbanColumnProps = {
	status: ContextStatus;
	count: number;
	children: ReactNode;
};

export function KanbanColumn({ status, count, children }: KanbanColumnProps) {
	const { t } = useTranslation();
	const { setNodeRef, isOver } = useDroppable({ id: status });

	return (
		<Stack gap="xs" style={{ minWidth: 260, flex: "1 0 260px" }}>
			<Group gap="xs" px={4}>
				<Text size="xs" fw={700} tt="uppercase" c="dimmed">
					{t(`contexts.kanban.status.${status}`)}
				</Text>
				<Badge size="sm" variant="light" color="gray">
					{count}
				</Badge>
			</Group>
			<Paper
				ref={setNodeRef}
				withBorder
				radius="md"
				p="xs"
				style={{
					flex: 1,
					minHeight: 120,
					backgroundColor: isOver
						? "var(--mantine-color-primary-light)"
						: "var(--mantine-color-dark-7)",
					transition: "background-color 120ms ease",
				}}
			>
				<Stack gap="xs">
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
