import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import type { BulkContextSpec } from "@/hooks/useContexts";
import type { BranchNamingPolicy } from "@/models/BranchNaming";

/**
 * Default preamble prefilled into the "bulk create from tasks" modal. It frames
 * the task text that follows so the AI coding agent knows the per-context
 * prompt describes one work item to implement end to end.
 */
export const DEFAULT_TASKS_PREAMBLE =
	"Implement the following task end to end. Read the title and description, " +
	"then make the changes needed to complete it. Keep the work scoped to this task.";

/**
 * Builds the per-context prompt stored on a context (its `preprompt`) from the
 * shared preamble and a single task. The preamble is written once by the user
 * and reused for every selected task; the task's own title and description are
 * appended so each context carries its specific work item.
 */
export function buildTaskPreprompt(preamble: string, task: WorkItem): string {
	const sections = [preamble.trim(), `Task: ${task.title.trim()}`];
	const description = task.description?.trim();
	if (description) {
		sections.push(`Description:\n${description}`);
	}
	if (task.url) {
		sections.push(`Link: ${task.url}`);
	}
	return sections.filter((section) => section.length > 0).join("\n\n");
}

/**
 * Maps selected Teamwork tasks into {@link BulkContextSpec}s, one context per
 * task. The context name is the task title; the branch name is the title run
 * through the project's {@link BranchNamingPolicy} so it is always a git-safe
 * ref. Tasks whose title slugifies to an empty branch name are dropped, since
 * a context cannot be created without a branch. The shared `preamble` plus the
 * task text becomes each context's `preprompt`.
 */
export function tasksToContextSpecs(
	tasks: readonly WorkItem[],
	preamble: string,
	branchNaming: BranchNamingPolicy,
): BulkContextSpec[] {
	return tasks.flatMap((task) => {
		const contextName = task.title.trim();
		if (contextName === "") return [];

		const branchName = branchNaming.composeBranchName(contextName, branchNaming.defaultPrefix);
		if (branchName === "") return [];

		return [{ contextName, branchName, preprompt: buildTaskPreprompt(preamble, task) }];
	});
}
