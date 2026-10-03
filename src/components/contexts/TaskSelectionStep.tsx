import {
	Alert,
	Button,
	Checkbox,
	Group,
	Loader,
	ScrollArea,
	Stack,
	Text,
	TextInput,
} from "@mantine/core";
import { InfoIcon, MagnifyingGlassIcon } from "@phosphor-icons/react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { LinkedTask } from "@/application/work-tracking/listLinkedProjectTasks";
import { translateError } from "@/components/work-tracking/translateError";
import type { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { taskKeyOf } from "@/models/bulk/taskContextDrafts";

type TaskSelectionStepProps = {
	tasks: readonly LinkedTask[];
	loading: boolean;
	error: WorkTrackingError | null;
	hasLinkedProjects: boolean;
	selectedKeys: ReadonlySet<string>;
	onSelectedKeysChange: (keys: ReadonlySet<string>) => void;
	onCancel: () => void;
	onNext: () => void;
};

export function TaskSelectionStep({
	tasks,
	loading,
	error,
	hasLinkedProjects,
	selectedKeys,
	onSelectedKeysChange,
	onCancel,
	onNext,
}: TaskSelectionStepProps) {
	const { t } = useTranslation();
	const [query, setQuery] = useState("");

	const visibleTasks = useMemo(() => {
		const normalizedQuery = query.toLowerCase().trim();
		if (!normalizedQuery) return tasks;
		return tasks.filter((task) => task.item.title.toLowerCase().includes(normalizedQuery));
	}, [tasks, query]);

	const visibleSelectedCount = visibleTasks.filter((task) =>
		selectedKeys.has(taskKeyOf(task)),
	).length;
	const allVisibleSelected =
		visibleTasks.length > 0 && visibleSelectedCount === visibleTasks.length;

	function toggleTask(task: LinkedTask) {
		const next = new Set(selectedKeys);
		const key = taskKeyOf(task);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		onSelectedKeysChange(next);
	}

	function toggleAllVisible() {
		const next = new Set(selectedKeys);
		for (const task of visibleTasks) {
			if (allVisibleSelected) next.delete(taskKeyOf(task));
			else next.add(taskKeyOf(task));
		}
		onSelectedKeysChange(next);
	}

	function renderContent() {
		if (!hasLinkedProjects) {
			return (
				<Alert icon={<InfoIcon size={16} />} color="gray" variant="light">
					{t("contexts.fromTasks.noLinkedProjects")}
				</Alert>
			);
		}
		if (error) {
			return (
				<Alert color="red" variant="light">
					{t("contexts.fromTasks.loadError", { message: translateError(t, error) })}
				</Alert>
			);
		}
		if (loading) {
			return (
				<Group justify="center" py="lg">
					<Loader size="sm" />
				</Group>
			);
		}
		if (tasks.length === 0) {
			return (
				<Text size="sm" c="dimmed">
					{t("contexts.fromTasks.noPendingTasks")}
				</Text>
			);
		}
		return (
			<Stack gap="xs">
				<TextInput
					placeholder={t("contexts.fromTasks.searchPlaceholder")}
					leftSection={<MagnifyingGlassIcon size={14} />}
					value={query}
					onChange={(event) => setQuery(event.currentTarget.value)}
					size="xs"
				/>
				<Checkbox
					label={t("contexts.fromTasks.selectAll")}
					checked={allVisibleSelected}
					indeterminate={visibleSelectedCount > 0 && !allVisibleSelected}
					onChange={toggleAllVisible}
					disabled={visibleTasks.length === 0}
				/>
				<ScrollArea.Autosize mah={360} type="auto">
					<Stack gap={6} pl="md">
						{visibleTasks.map((task) => (
							<Checkbox
								key={taskKeyOf(task)}
								label={task.item.title}
								checked={selectedKeys.has(taskKeyOf(task))}
								onChange={() => toggleTask(task)}
							/>
						))}
					</Stack>
				</ScrollArea.Autosize>
			</Stack>
		);
	}

	return (
		<Stack gap="md">
			<Text size="sm" c="dimmed">
				{t("contexts.fromTasks.selectDescription")}
			</Text>
			{renderContent()}
			<Group justify="flex-end">
				<Button variant="subtle" onClick={onCancel}>
					{t("common.cancel")}
				</Button>
				<Button onClick={onNext} disabled={selectedKeys.size === 0}>
					{t("contexts.fromTasks.next", { count: selectedKeys.size })}
				</Button>
			</Group>
		</Stack>
	);
}
