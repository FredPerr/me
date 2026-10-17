import {
	Button,
	Checkbox,
	Group,
	Modal,
	ScrollArea,
	Select,
	Stack,
	Text,
	Textarea,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProjectView, TaskToAssignInput } from "@/bindings";
import { useAssignMeToTasksSetting } from "@/hooks/settings/useAssignMeToTasksSetting";
import { useAssignMeToTasks } from "@/hooks/work-tracking/useAssignMeToTasks";
import { useProjectPendingTasks } from "@/hooks/work-tracking/useProjectPendingTasks";
import { useCreateContexts } from "@/hooks/workspace/useCreateContexts";
import {
	draftTaskContext,
	type TaskContextDraft,
	taskKeyOf,
	toBulkContextSpec,
	validateTaskContextDrafts,
} from "@/models/bulk/taskContextDrafts";
import { buildTaskPreprompt, DEFAULT_TASKS_PREAMBLE } from "@/models/bulk/tasksToContextSpecs";
import { findContext, findDefaultContext } from "@/models/projectView";
import { TaskContextDraftCard } from "./TaskContextDraftCard";
import { TaskSelectionStep } from "./TaskSelectionStep";

type Step = "select" | "configure";

type CreateContextsFromTasksModalProps = {
	project: ProjectView;
	onClose: () => void;
};

export function CreateContextsFromTasksModal({
	project,
	onClose,
}: CreateContextsFromTasksModalProps) {
	const { t } = useTranslation();
	const { contexts } = project;
	const hasLinkedProjects = project.remoteProjectLinks.length > 0;
	const pendingTaskCards = useProjectPendingTasks(project.tag, hasLinkedProjects);
	const [step, setStep] = useState<Step>("select");
	const [selectedKeys, setSelectedKeys] = useState<ReadonlySet<string>>(new Set());
	const [draftsByKey, setDraftsByKey] = useState<ReadonlyMap<string, TaskContextDraft>>(new Map());
	const [instruction, setInstruction] = useState(DEFAULT_TASKS_PREAMBLE);
	const { assignMeToTasks, setAssignMeToTasks } = useAssignMeToTasksSetting();
	const [selectedBaseContextId, setSelectedBaseContextId] = useState<string | null>(null);
	const { createContexts, creating } = useCreateContexts(project);
	const { mutateAsync: assignMeToTracker } = useAssignMeToTasks();

	const pendingTasks = pendingTaskCards.tasks;

	const defaultBaseContext = findDefaultContext(project) ?? contexts[0];
	const baseContextId = selectedBaseContextId ?? defaultBaseContext?.id ?? null;
	const baseContext = baseContextId ? findContext(project, baseContextId) : undefined;

	const validatedDrafts = useMemo(() => {
		const drafts = pendingTasks.flatMap((task) => draftsByKey.get(taskKeyOf(task)) ?? []);
		return validateTaskContextDrafts(drafts, contexts);
	}, [pendingTasks, draftsByKey, contexts]);

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
			nextDrafts.set(key, draftsByKey.get(key) ?? draftTaskContext(task, instruction));
		}
		setDraftsByKey(nextDrafts);
		setStep("configure");
	}

	function applyInstruction(nextInstruction: string) {
		setInstruction(nextInstruction);
		setDraftsByKey((current) => {
			const next = new Map<string, TaskContextDraft>();
			for (const [key, draft] of current) {
				next.set(key, { ...draft, prompt: buildTaskPreprompt(nextInstruction, draft.task.task) });
			}
			return next;
		});
	}

	function updateDraft(draft: TaskContextDraft) {
		setDraftsByKey((current) => new Map(current).set(taskKeyOf(draft.task), draft));
	}

	/**
	 * Adds the current user to each selected remote task in one best-effort call.
	 * Runs after the local contexts are created, so a tracker hiccup is surfaced
	 * as a warning that names the tasks that could not be assigned, without
	 * undoing the created contexts.
	 */
	async function assignMeToSelectedTasks() {
		const tasks: TaskToAssignInput[] = validatedDrafts.map(({ draft }) => ({
			connectionId: draft.task.connectionId,
			taskId: draft.task.task.id,
		}));
		const result = await assignMeToTracker(tasks);
		if (result.failed.length === 0) return;

		const failedTitles = result.failed
			.map((failure) => titleOfTask(failure.connectionId, failure.taskId))
			.join(", ");
		notifications.show({
			title: t("contexts.fromTasks.assignMeFailedTitle"),
			message: t("contexts.fromTasks.assignMeFailedMessage", {
				count: result.failed.length,
				tasks: failedTitles,
			}),
			color: "yellow",
		});
	}

	/** The title of a selected task, so a failure names the task, not its id. */
	function titleOfTask(connectionId: string, taskId: string): string {
		const match = validatedDrafts.find(
			({ draft }) => draft.task.connectionId === connectionId && draft.task.task.id === taskId,
		);
		return match?.draft.contextName ?? taskId;
	}

	function toggleAssignMe(checked: boolean) {
		setAssignMeToTasks(checked);
	}

	async function handleCreate() {
		if (!baseContext || !canCreate) return;
		try {
			await createContexts(validatedDrafts.map(toBulkContextSpec), baseContext);
		} catch {
			return;
		}
		notifications.show({
			title: t("contexts.fromTasks.createdTitle"),
			message: t("contexts.fromTasks.createdMessage", { count: validatedDrafts.length }),
			color: "green",
		});
		if (assignMeToTasks) {
			await assignMeToSelectedTasks();
		}
		onClose();
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
					loading={pendingTaskCards.loading}
					error={pendingTaskCards.error}
					unavailableConnections={pendingTaskCards.unavailableConnections}
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
					<Textarea
						label={t("workTracking.createFromTask.sharedInstructionLabel")}
						description={t("workTracking.createFromTask.sharedInstructionHint")}
						value={instruction}
						onChange={(event) => applyInstruction(event.currentTarget.value)}
						autosize
						minRows={2}
						maxRows={6}
						disabled={creating}
					/>
					<Checkbox
						label={t("contexts.fromTasks.assignMeLabel")}
						description={t("contexts.fromTasks.assignMeHint")}
						checked={assignMeToTasks}
						onChange={(event) => toggleAssignMe(event.currentTarget.checked)}
						disabled={creating}
					/>
					<ScrollArea.Autosize mah="55vh" type="auto">
						<Stack gap="sm">
							{validatedDrafts.map((validatedDraft) => (
								<TaskContextDraftCard
									key={taskKeyOf(validatedDraft.draft.task)}
									project={project}
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
