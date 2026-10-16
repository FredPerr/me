import type { LinkedTask } from "@/application/work-tracking/listLinkedProjectTasks";
import { taskRefOf } from "@/application/work-tracking/selectPendingTasks";
import type { ContextView } from "@/bindings";
import type { BulkContextSpec } from "./BulkContextSpec";
import { buildTaskPreprompt, DEFAULT_TASKS_PREAMBLE } from "./tasksToContextSpecs";

export type TaskContextDraft = {
	readonly task: LinkedTask;
	readonly contextName: string;
	readonly prompt: string;
};

type TaskContextDraftIssue = "nameRequired" | "duplicateName";

export type ValidatedTaskContextDraft = {
	readonly draft: TaskContextDraft;
	readonly issue?: TaskContextDraftIssue;
};

export function taskKeyOf(task: LinkedTask): string {
	return `${task.connectionId}|${task.item.id}`;
}

export function draftTaskContext(
	task: LinkedTask,
	preamble: string = DEFAULT_TASKS_PREAMBLE,
): TaskContextDraft {
	return {
		task,
		contextName: task.item.title.trim(),
		prompt: buildTaskPreprompt(preamble, task.item),
	};
}

function normalizeName(name: string): string {
	return name.trim().toLowerCase();
}

function countOccurrences(values: readonly string[]): Map<string, number> {
	const counts = new Map<string, number>();
	for (const value of values) counts.set(value, (counts.get(value) ?? 0) + 1);
	return counts;
}

/**
 * Flags the drafts that can't all be created in one go: an empty name, or a
 * name used twice in the batch or by an existing context. The backend still
 * checks every context it creates.
 */
export function validateTaskContextDrafts(
	drafts: readonly TaskContextDraft[],
	existingContexts: readonly ContextView[],
): ValidatedTaskContextDraft[] {
	const existingNames = new Set(existingContexts.map((context) => normalizeName(context.name)));
	const nameCounts = countOccurrences(drafts.map((draft) => normalizeName(draft.contextName)));

	function issueOf(name: string): TaskContextDraftIssue | undefined {
		if (!name) return "nameRequired";
		if (existingNames.has(name) || (nameCounts.get(name) ?? 0) > 1) return "duplicateName";
		return undefined;
	}

	return drafts.map((draft) => ({ draft, issue: issueOf(normalizeName(draft.contextName)) }));
}

export function toBulkContextSpec({ draft }: ValidatedTaskContextDraft): BulkContextSpec {
	return {
		contextName: draft.contextName.trim(),
		agentPrompt: draft.prompt.trim() || undefined,
		task: taskRefOf(draft.task),
	};
}
