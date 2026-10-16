import { describe, expect, it } from "vitest";
import { toWorkItemId, toWorkProjectId } from "@/domain/work-tracking/identifiers";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import { WorkItemPriority } from "@/domain/work-tracking/WorkItemPriority";
import { WorkItemStatus } from "@/domain/work-tracking/WorkItemStatus";
import {
	buildTaskPreprompt,
	DEFAULT_TASKS_PREAMBLE,
	tasksToContextSpecs,
} from "./tasksToContextSpecs";

const TASK_TITLE = "Build the login page";
const TASK_DESCRIPTION = "Add email and password fields with validation.";
const TASK_URL = "https://example.teamwork.com/tasks/42";
const PREAMBLE = "Do the work described below.";
const CONNECTION_ID = "teamwork:acme.teamwork.com";
const TASK_ID = "task-1";

function buildTask(overrides?: Partial<WorkItem>): WorkItem {
	return {
		id: toWorkItemId(TASK_ID),
		projectId: toWorkProjectId("project-1"),
		title: TASK_TITLE,
		description: TASK_DESCRIPTION,
		status: WorkItemStatus.Todo,
		rawStatus: "new",
		priority: WorkItemPriority.None,
		assignees: [],
		labels: [],
		url: TASK_URL,
		...overrides,
	};
}

describe("buildTaskPreprompt", () => {
	it("combines the preamble, title, description and link", () => {
		const result = buildTaskPreprompt(PREAMBLE, buildTask());

		expect(result).toContain(PREAMBLE);
		expect(result).toContain(`Task: ${TASK_TITLE}`);
		expect(result).toContain(`Description:\n${TASK_DESCRIPTION}`);
		expect(result).toContain(`Link: ${TASK_URL}`);
	});

	it("omits the description section when the task has none", () => {
		const result = buildTaskPreprompt(PREAMBLE, buildTask({ description: undefined }));

		expect(result).not.toContain("Description:");
		expect(result).toContain(`Task: ${TASK_TITLE}`);
	});

	it("omits the link section when the task has no url", () => {
		const result = buildTaskPreprompt(PREAMBLE, buildTask({ url: "" }));

		expect(result).not.toContain("Link:");
	});
});

describe("tasksToContextSpecs", () => {
	it("creates one spec per task with the title as context name", () => {
		const specs = tasksToContextSpecs([buildTask()], PREAMBLE, CONNECTION_ID);

		expect(specs).toHaveLength(1);
		expect(specs[0].contextName).toBe(TASK_TITLE);
	});

	it("remembers the task each context is created from", () => {
		const specs = tasksToContextSpecs([buildTask()], PREAMBLE, CONNECTION_ID);

		expect(specs[0].task).toEqual({ connectionId: CONNECTION_ID, taskId: TASK_ID });
	});

	it("stores the composed preamble and task text as the agent prompt", () => {
		const specs = tasksToContextSpecs([buildTask()], PREAMBLE, CONNECTION_ID);

		expect(specs[0].agentPrompt).toContain(PREAMBLE);
		expect(specs[0].agentPrompt).toContain(TASK_TITLE);
	});

	it("drops tasks without a title", () => {
		const specs = tasksToContextSpecs([buildTask({ title: "   " })], PREAMBLE, CONNECTION_ID);

		expect(specs).toHaveLength(0);
	});

	it("maps several tasks into several specs", () => {
		const tasks = [
			buildTask({ id: toWorkItemId("a"), title: "First task" }),
			buildTask({ id: toWorkItemId("b"), title: "Second task" }),
		];

		const specs = tasksToContextSpecs(tasks, PREAMBLE, CONNECTION_ID);

		expect(specs.map((spec) => spec.contextName)).toEqual(["First task", "Second task"]);
	});

	it("exposes a non-empty default preamble", () => {
		expect(DEFAULT_TASKS_PREAMBLE.trim().length).toBeGreaterThan(0);
	});
});
