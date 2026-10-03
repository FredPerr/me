import { Badge, Card, Flex, Text } from "@mantine/core";
import { ListChecksIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { LinkedTask } from "@/application/work-tracking/listLinkedProjectTasks";

type TaskCardProps = {
	task: LinkedTask;
	onOpen: (task: LinkedTask) => void;
};

export function TaskCard({ task, onOpen }: TaskCardProps) {
	const { t } = useTranslation();

	return (
		<Card
			component="button"
			type="button"
			withBorder
			padding="xs"
			onClick={() => onOpen(task)}
			aria-label={t("contexts.kanban.taskCardLabel", { title: task.item.title })}
			style={{
				borderStyle: "dashed",
				borderColor: "var(--mantine-color-gray-6)",
				backgroundColor: "transparent",
				cursor: "pointer",
				textAlign: "left",
				width: "100%",
			}}
		>
			<Flex gap="xs" align="center" justify="space-between" wrap="nowrap">
				<Flex gap="xs" align="center" style={{ minWidth: 0 }}>
					<ListChecksIcon size={14} color="var(--mantine-color-dimmed)" style={{ flexShrink: 0 }} />
					<Text size="xs" fw={500} c="dimmed" truncate>
						{task.item.title}
					</Text>
				</Flex>
				<Badge size="xs" variant="outline" color="gray" style={{ flexShrink: 0 }}>
					{t("contexts.kanban.taskBadge")}
				</Badge>
			</Flex>
		</Card>
	);
}
