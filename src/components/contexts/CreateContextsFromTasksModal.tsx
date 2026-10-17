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
import { selectPendingTasks } from "@/application/work-tracking/selectPendingTasks";
import { commands, type ProjectView } from "@/bindings";
import { useLinkedProjectTasks } from "@/hooks/work-tracking/useLinkedProjectTasks";
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
	const linkedTasks = useLinkedProjectTasks(project.remoteProjectLinks, hasLinkedProjects);
	const [step, setStep] = useState<Step>("select");
	const [selectedKeys, setSelectedKeys] = useState<ReadonlySet<string>>(new Set());
	const [draftsByKey, setDraftsByKey] = useState<ReadonlyMap<string, TaskContextDraft>>(new Map());
	const [instruction, setInstruction] = useState(DEFAULT_TASKS_PREAMBLE);
	const [assignMe, setAssignMe] = useState(false);
	const [selectedBaseContextId, setSelectedBaseContextId] = useState<string | null>(null);
	const { createContexts, creating } = useCreateContexts(project);

	const pendingTasks = useMemo(
		() => selectPendingTasks(linkedTasks.tasks, contexts),
		[linkedTasks.tasks, contexts],
	);

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
	 * Adds the current user to each selected remote task. Runs after the local
	 * contexts are created, so a provider failure here is surfaced as a warning
	 * without undoing the created contexts.
	 */
	async function assignMeToSelectedTasks() {
		const results = await Promise.allSettled(
			validatedDrafts.map(({ draft }) =>
				commands.workTrackingAssignMeToTask(draft.task.connectionId, draft.task.task.id),
			),
		);
		const failed = results.filter((result) => result.status === "rejected").length;
		if (failed > 0) {
			notifications.show({
				title: t("contexts.fromTasks.assignMeFailedTitle"),
				message: t("contexts.fromTasks.assignMeFailedMessage", { count: failed }),
				color: "yellow",
			});
		}
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
		if (assignMe) {
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
						checked={assignMe}
						onChange={(event) => setAssignMe(event.currentTarget.checked)}
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
