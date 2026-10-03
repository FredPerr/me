import { Button, Group, Modal, ScrollArea, Select, Stack, Text } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { selectPendingTasks } from "@/application/work-tracking/selectPendingTasks";
import type { BulkContextSpec } from "@/hooks/useContexts";
import { useLinkedProjectTasks } from "@/hooks/work-tracking/useLinkedProjectTasks";
import {
	draftTaskContext,
	type TaskContextDraft,
	taskKeyOf,
	toBulkContextSpec,
	validateTaskContextDrafts,
} from "@/models/bulk/taskContextDrafts";
import type { Context, Project } from "@/models/Project";
import { TaskContextDraftCard } from "./TaskContextDraftCard";
import { TaskSelectionStep } from "./TaskSelectionStep";

type Step = "select" | "configure";

type CreateContextsFromTasksModalProps = {
	project: Project;
	contexts: readonly Context[];
	onClose: () => void;
	onCreate: (specs: BulkContextSpec[], baseContext: Context) => Promise<void>;
};

export function CreateContextsFromTasksModal({
	project,
	contexts,
	onClose,
	onCreate,
}: CreateContextsFromTasksModalProps) {
	const { t } = useTranslation();
	const hasLinkedProjects = project.remoteProjectLinks.toArray().length > 0;
	const linkedTasks = useLinkedProjectTasks(project.remoteProjectLinks, hasLinkedProjects);
	const [step, setStep] = useState<Step>("select");
	const [selectedKeys, setSelectedKeys] = useState<ReadonlySet<string>>(new Set());
	const [draftsByKey, setDraftsByKey] = useState<ReadonlyMap<string, TaskContextDraft>>(new Map());
	const [selectedBaseContextId, setSelectedBaseContextId] = useState<string | null>(null);
	const [creating, setCreating] = useState(false);

	const pendingTasks = useMemo(
		() => selectPendingTasks(linkedTasks.tasks, contexts),
		[linkedTasks.tasks, contexts],
	);

	const defaultBaseContext = contexts.find((context) => context.isDefault) ?? contexts[0];
	const baseContextId = selectedBaseContextId ?? defaultBaseContext?.id ?? null;
	const baseContext = contexts.find((context) => context.id === baseContextId);

	const validatedDrafts = useMemo(() => {
		const drafts = pendingTasks.flatMap((task) => draftsByKey.get(taskKeyOf(task)) ?? []);
		return validateTaskContextDrafts(drafts, project.branchNaming, contexts);
	}, [pendingTasks, draftsByKey, project.branchNaming, contexts]);

	const canCreate =
		Boolean(baseContext) &&
		validatedDrafts.length > 0 &&
		validatedDrafts.every((validated) => !validated.issue) &&
		!creating;

	function handleClose() {
		if (!creating) onClose();
	}

	function goToConfigure() {
		const nextDrafts = new Map<string, TaskContextDraft>();
		for (const task of pendingTasks) {
			const key = taskKeyOf(task);
			if (!selectedKeys.has(key)) continue;
			nextDrafts.set(key, draftsByKey.get(key) ?? draftTaskContext(task));
		}
		setDraftsByKey(nextDrafts);
		setStep("configure");
	}

	function updateDraft(draft: TaskContextDraft) {
		setDraftsByKey((current) => new Map(current).set(taskKeyOf(draft.task), draft));
	}

	async function handleCreate() {
		if (!baseContext || !canCreate) return;
		setCreating(true);
		try {
			await onCreate(validatedDrafts.map(toBulkContextSpec), baseContext);
			notifications.show({
				title: t("contexts.fromTasks.createdTitle"),
				message: t("contexts.fromTasks.createdMessage", { count: validatedDrafts.length }),
				color: "green",
			});
			onClose();
		} catch (error) {
			notifications.show({
				title: t("contexts.fromTasks.createFailed"),
				message: String(error),
				color: "red",
			});
		} finally {
			setCreating(false);
		}
	}

	return (
		<Modal
			opened
			onClose={handleClose}
			title={t("contexts.fromTasks.title")}
			size="lg"
			closeOnClickOutside={!creating}
		>
			{step === "select" ? (
				<TaskSelectionStep
					tasks={pendingTasks}
					loading={linkedTasks.loading}
					error={linkedTasks.error}
					hasLinkedProjects={hasLinkedProjects}
					selectedKeys={selectedKeys}
					onSelectedKeysChange={setSelectedKeys}
					onCancel={handleClose}
					onNext={goToConfigure}
				/>
			) : (
				<Stack gap="md">
					<Text size="sm" c="dimmed">
						{t("contexts.fromTasks.configureDescription")}
					</Text>
					<Select
						label={t("workTracking.createFromTask.baseContextLabel")}
						description={t("workTracking.createFromTask.baseContextHint")}
						data={contexts.map((context) => ({
							value: context.id,
							label: context.isDefault ? t("common.default") : context.name,
						}))}
						value={baseContextId}
						onChange={setSelectedBaseContextId}
						allowDeselect={false}
						disabled={creating}
						searchable
					/>
					<ScrollArea.Autosize mah="55vh" type="auto">
						<Stack gap="sm">
							{validatedDrafts.map((validatedDraft) => (
								<TaskContextDraftCard
									key={taskKeyOf(validatedDraft.draft.task)}
									validatedDraft={validatedDraft}
									disabled={creating}
									onChange={updateDraft}
								/>
							))}
						</Stack>
					</ScrollArea.Autosize>
					<Group justify="space-between">
						<Button variant="subtle" onClick={() => setStep("select")} disabled={creating}>
							{t("common.back")}
						</Button>
						<Group gap="xs">
							<Button variant="subtle" onClick={handleClose} disabled={creating}>
								{t("common.cancel")}
							</Button>
							<Button onClick={handleCreate} loading={creating} disabled={!canCreate}>
								{t("contexts.fromTasks.createButton", { count: validatedDrafts.length })}
							</Button>
						</Group>
					</Group>
				</Stack>
			)}
		</Modal>
	);
}
