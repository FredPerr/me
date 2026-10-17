import { ActionIcon, Avatar, Badge, Checkbox, Group, Paper, Stack, Text } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import type { ConnectionView, Person, Task, TaskPriority, TaskStatus } from "@/bindings";
import { isOpenableProviderUrl } from "@/hooks/work-tracking/providerUrl";
import { formatDueDate } from "./formatDueDate";

const STATUS_LABEL_KEYS: Record<TaskStatus, string> = {
	todo: "workTracking.status.todo",
	inProgress: "workTracking.status.inProgress",
	done: "workTracking.status.done",
	cancelled: "workTracking.status.cancelled",
	unknown: "workTracking.status.unknown",
};

const STATUS_COLORS: Record<TaskStatus, string> = {
	todo: "gray",
	inProgress: "blue",
	done: "green",
	cancelled: "dark",
	unknown: "gray",
};

const PRIORITY_LABEL_KEYS: Record<TaskPriority, string> = {
	none: "workTracking.priority.none",
	low: "workTracking.priority.low",
	medium: "workTracking.priority.medium",
	high: "workTracking.priority.high",
	urgent: "workTracking.priority.urgent",
	unknown: "workTracking.priority.unknown",
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

function httpsOrUndefined(url: string | null): string | undefined {
	return url?.startsWith("https://") ? url : undefined;
}

type WorkItemRowProps = {
	task: Task;
	connection: ConnectionView;
	isOrphanSubtask: boolean;
	selectable?: boolean;
	selected?: boolean;
	onToggleSelected?: (task: Task, selected: boolean) => void;
	/** Shows a short, truncated description under the title when available. */
	showDescription?: boolean;
};

export function WorkItemRow({
	task,
	connection,
	isOrphanSubtask,
	selectable = false,
	selected = false,
	onToggleSelected,
	showDescription = false,
}: WorkItemRowProps) {
	const { t, i18n } = useTranslation();
	const dueDate = formatDueDate(task.dueDate, i18n.language);
	const canOpen = isOpenableProviderUrl(task.url, connection.baseUrl);

	return (
		<Paper withBorder p="xs" radius="sm" mb={4}>
			<Group justify="space-between" wrap="nowrap" align="flex-start" gap="xs">
				{selectable && (
					<Checkbox
						checked={selected}
						onChange={(event) => onToggleSelected?.(task, event.currentTarget.checked)}
						aria-label={t("workTracking.bulkCreate.selectTask", { title: task.title })}
						mt={2}
					/>
				)}
				<Stack gap={4} style={{ minWidth: 0, flex: 1 }}>
					<Text size="sm" fw={500}>
						{task.title}
					</Text>
					{showDescription && task.description && (
						<Text size="xs" c="dimmed" lineClamp={2}>
							{task.description}
						</Text>
					)}
					<Group gap={6}>
						<Badge variant="light" size="sm" color={STATUS_COLORS[task.status]}>
							{t(STATUS_LABEL_KEYS[task.status])}
						</Badge>
						{task.priority !== "none" && (
							<Badge variant="outline" size="sm" color="orange">
								{t(PRIORITY_LABEL_KEYS[task.priority])}
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
					{task.labels.length > 0 && (
						<Group gap={4}>
							{task.labels.map((label) => {
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
					{task.assignees.length > 0 && <Assignees assignees={task.assignees} />}
				</Stack>
				{canOpen && (
					<ActionIcon
						component="a"
						href={task.url}
						variant="subtle"
						aria-label={t("workTracking.items.openInProvider", { title: task.title })}
						onClick={(event) => {
							event.preventDefault();
							void openUrl(task.url);
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
