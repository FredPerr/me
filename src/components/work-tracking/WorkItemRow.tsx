import { ActionIcon, Avatar, Badge, Group, Paper, Stack, Text } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import type { Person } from "@/domain/work-tracking/Person";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { isOpenableProviderUrl } from "@/domain/work-tracking/providerUrl";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import { WorkItemPriority } from "@/domain/work-tracking/WorkItemPriority";
import { WorkItemStatus } from "@/domain/work-tracking/WorkItemStatus";
import { formatDueDate } from "./formatDueDate";

const STATUS_LABEL_KEYS: Record<WorkItemStatus, string> = {
	[WorkItemStatus.Todo]: "workTracking.status.todo",
	[WorkItemStatus.InProgress]: "workTracking.status.inProgress",
	[WorkItemStatus.Done]: "workTracking.status.done",
	[WorkItemStatus.Cancelled]: "workTracking.status.cancelled",
	[WorkItemStatus.Unknown]: "workTracking.status.unknown",
};

const STATUS_COLORS: Record<WorkItemStatus, string> = {
	[WorkItemStatus.Todo]: "gray",
	[WorkItemStatus.InProgress]: "blue",
	[WorkItemStatus.Done]: "green",
	[WorkItemStatus.Cancelled]: "dark",
	[WorkItemStatus.Unknown]: "gray",
};

const PRIORITY_LABEL_KEYS: Record<WorkItemPriority, string> = {
	[WorkItemPriority.None]: "workTracking.priority.none",
	[WorkItemPriority.Low]: "workTracking.priority.low",
	[WorkItemPriority.Medium]: "workTracking.priority.medium",
	[WorkItemPriority.High]: "workTracking.priority.high",
	[WorkItemPriority.Urgent]: "workTracking.priority.urgent",
	[WorkItemPriority.Unknown]: "workTracking.priority.unknown",
};

// Same rule as the backend Label constructor; checked again because the value becomes CSS.
const LABEL_COLOR_PATTERN = /^#[0-9a-fA-F]{3,8}$/;
const MAX_VISIBLE_AVATARS = 3;

function initialsOf(displayName: string): string {
	return displayName
		.split(/\s+/)
		.filter((part) => part.length > 0)
		.slice(0, 2)
		.map((part) => part[0]?.toUpperCase())
		.join("");
}

function httpsOrUndefined(url: string | undefined): string | undefined {
	return url?.startsWith("https://") ? url : undefined;
}

type WorkItemRowProps = {
	item: WorkItem;
	connection: ProviderConnection;
	isOrphanSubtask: boolean;
};

export function WorkItemRow({ item, connection, isOrphanSubtask }: WorkItemRowProps) {
	const { t, i18n } = useTranslation();
	const dueDate = formatDueDate(item.dueDate, i18n.language);
	const canOpen = isOpenableProviderUrl(item.url, connection.baseUrl);

	return (
		<Paper withBorder p="xs" radius="sm" mb={4}>
			<Group justify="space-between" wrap="nowrap" align="flex-start" gap="xs">
				<Stack gap={4} style={{ minWidth: 0 }}>
					<Text size="sm" fw={500}>
						{item.title}
					</Text>
					<Group gap={6}>
						<Badge variant="light" size="sm" color={STATUS_COLORS[item.status]}>
							{t(STATUS_LABEL_KEYS[item.status])}
						</Badge>
						{item.priority !== WorkItemPriority.None && (
							<Badge variant="outline" size="sm" color="orange">
								{t(PRIORITY_LABEL_KEYS[item.priority])}
							</Badge>
						)}
						{isOrphanSubtask && (
							<Badge variant="outline" size="sm" color="gray">
								{t("workTracking.items.subtask")}
							</Badge>
						)}
						{dueDate && (
							<Text size="xs" c="dimmed">
								{t("workTracking.items.due", { date: dueDate })}
							</Text>
						)}
					</Group>
					{item.labels.length > 0 && (
						<Group gap={4}>
							{item.labels.map((label) => {
								const color =
									label.color && LABEL_COLOR_PATTERN.test(label.color) ? label.color : undefined;
								return (
									<Badge
										key={label.name}
										variant="dot"
										size="xs"
										style={color ? { "--badge-dot-color": color } : undefined}
									>
										{label.name}
									</Badge>
								);
							})}
						</Group>
					)}
					{item.assignees.length > 0 && <Assignees assignees={item.assignees} />}
				</Stack>
				{canOpen && (
					<ActionIcon
						component="a"
						href={item.url}
						variant="subtle"
						aria-label={t("workTracking.items.openInProvider", { title: item.title })}
						onClick={(event) => {
							event.preventDefault();
							void openUrl(item.url);
						}}
					>
						<ArrowSquareOutIcon size={16} />
					</ActionIcon>
				)}
			</Group>
		</Paper>
	);
}

function Assignees({ assignees }: { assignees: readonly Person[] }) {
	const { t } = useTranslation();
	const names = assignees.map((person) => person.displayName).join(", ");
	const visible = assignees.slice(0, MAX_VISIBLE_AVATARS);
	const hiddenCount = assignees.length - visible.length;

	return (
		<Group gap={6} wrap="nowrap">
			<Avatar.Group spacing="xs" aria-hidden="true">
				{visible.map((person) => (
					<Avatar
						key={person.id}
						src={httpsOrUndefined(person.avatarUrl)}
						alt=""
						size="sm"
						radius="xl"
					>
						{initialsOf(person.displayName)}
					</Avatar>
				))}
				{hiddenCount > 0 && (
					<Avatar size="sm" radius="xl">
						+{hiddenCount}
					</Avatar>
				)}
			</Avatar.Group>
			<Text size="xs" c="dimmed" truncate>
				{t("workTracking.items.assignees", { names })}
			</Text>
		</Group>
	);
}
