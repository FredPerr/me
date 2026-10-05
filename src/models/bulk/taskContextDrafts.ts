import type { LinkedTask } from "@/application/work-tracking/listLinkedProjectTasks";
import { workItemRefOf } from "@/application/work-tracking/selectPendingTasks";
import type { BulkContextSpec } from "@/hooks/useContexts";
import type { BranchNamingPolicy } from "@/models/BranchNaming";
import type { Context } from "@/models/Project";
import { buildTaskPreprompt, DEFAULT_TASKS_PREAMBLE } from "./tasksToContextSpecs";

export type TaskContextDraft = {
	readonly task: LinkedTask;
	readonly contextName: string;
	readonly prompt: string;
};

type TaskContextDraftIssue = "nameRequired" | "invalidBranch" | "duplicateName" | "duplicateBranch";

export type ValidatedTaskContextDraft = {
	readonly draft: TaskContextDraft;
	readonly branchName: string;
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
 * Derives each draft's branch and flags the issues that would make its creation fail: an empty
 * name, a name that slugifies to no branch, or a name/branch clashing with another draft or an
 * existing context (worktree folders are compared case-insensitively).
 */
export function validateTaskContextDrafts(
	drafts: readonly TaskContextDraft[],
	branchNaming: BranchNamingPolicy,
	existingContexts: readonly Context[],
): ValidatedTaskContextDraft[] {
	const branchNames = drafts.map((draft) => {
		const name = draft.contextName.trim();
		return name ? branchNaming.composeBranchName(name, branchNaming.defaultPrefix) : "";
	});
	const existingNames = new Set(existingContexts.map((context) => normalizeName(context.name)));
	const nameCounts = countOccurrences(drafts.map((draft) => normalizeName(draft.contextName)));
	const branchCounts = countOccurrences(branchNames);

	function issueOf(name: string, branchName: string): TaskContextDraftIssue | undefined {
		if (!name) return "nameRequired";
		if (!branchName) return "invalidBranch";
		if (existingNames.has(name) || (nameCounts.get(name) ?? 0) > 1) return "duplicateName";
		if ((branchCounts.get(branchName) ?? 0) > 1) return "duplicateBranch";
		return undefined;
	}

	return drafts.map((draft, index) => {
		const branchName = branchNames[index];
		return { draft, branchName, issue: issueOf(normalizeName(draft.contextName), branchName) };
	});
}

export function toBulkContextSpec({
	draft,
	branchName,
}: ValidatedTaskContextDraft): BulkContextSpec {
	return {
		contextName: draft.contextName.trim(),
		branchName,
		preprompt: draft.prompt.trim() || undefined,
		workItemRef: workItemRefOf(draft.task),
	};
}
