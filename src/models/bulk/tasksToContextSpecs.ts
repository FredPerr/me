import type { Task } from "@/bindings";
import type { BulkContextSpec } from "./BulkContextSpec";

/**
 * Default preamble prefilled into the "bulk create from tasks" modal. It frames
 * the task text that follows so the AI coding agent knows the per-context
 * prompt describes one work item to implement end to end.
 */
export const DEFAULT_TASKS_PREAMBLE =
	"Implement the following task end to end. Read the title and description, " +
	"then make the changes needed to complete it. Keep the work scoped to this task.";

/**
 * Builds the agent prompt stored on a context from the
 * shared preamble and a single task. The preamble is written once by the user
 * and reused for every selected task; the task's own title and description are
 * appended so each context carries its specific work item.
 */
export function buildTaskPreprompt(preamble: string, task: Task): string {
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
 * Maps selected tasks into {@link BulkContextSpec}s, one context per task
 * created from it. The context name is the task title; its branch is named
 * after it by the project. Tasks without a title are dropped. The shared
 * `preamble` plus the task text becomes each context's agent prompt.
 */
export function tasksToContextSpecs(
	tasks: readonly Task[],
	preamble: string,
	connectionId: string,
): BulkContextSpec[] {
	return tasks.flatMap((task) => {
		const contextName = task.title.trim();
		if (contextName === "") return [];
		return [
			{
				contextName,
				agentPrompt: buildTaskPreprompt(preamble, task),
				task: { connectionId, taskId: task.id },
			},
		];
	});
}
