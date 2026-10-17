import { describe, expect, it } from "vitest";
import { buildTask } from "@/test-utils/workTrackingBuilders";
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

function buildTaskWithText(overrides?: Parameters<typeof buildTask>[0]) {
	return buildTask({
		id: TASK_ID,
		title: TASK_TITLE,
		description: TASK_DESCRIPTION,
		url: TASK_URL,
		...overrides,
	});
}

describe("buildTaskPreprompt", () => {
	it("combines the preamble, title, description and link", () => {
		const result = buildTaskPreprompt(PREAMBLE, buildTaskWithText());

		expect(result).toContain(PREAMBLE);
		expect(result).toContain(`Task: ${TASK_TITLE}`);
		expect(result).toContain(`Description:\n${TASK_DESCRIPTION}`);
		expect(result).toContain(`Link: ${TASK_URL}`);
	});

	it("omits the description section when the task has none", () => {
		const result = buildTaskPreprompt(PREAMBLE, buildTaskWithText({ description: null }));

		expect(result).not.toContain("Description:");
		expect(result).toContain(`Task: ${TASK_TITLE}`);
	});

	it("omits the link section when the task has no url", () => {
		const result = buildTaskPreprompt(PREAMBLE, buildTaskWithText({ url: "" }));

		expect(result).not.toContain("Link:");
	});
});

describe("tasksToContextSpecs", () => {
	it("creates one spec per task with the title as context name", () => {
		const specs = tasksToContextSpecs([buildTaskWithText()], PREAMBLE, CONNECTION_ID);

		expect(specs).toHaveLength(1);
		expect(specs[0].contextName).toBe(TASK_TITLE);
	});

	it("remembers the task each context is created from", () => {
		const specs = tasksToContextSpecs([buildTaskWithText()], PREAMBLE, CONNECTION_ID);

		expect(specs[0].task).toEqual({ connectionId: CONNECTION_ID, taskId: TASK_ID });
	});

	it("stores the composed preamble and task text as the agent prompt", () => {
		const specs = tasksToContextSpecs([buildTaskWithText()], PREAMBLE, CONNECTION_ID);

		expect(specs[0].agentPrompt).toContain(PREAMBLE);
		expect(specs[0].agentPrompt).toContain(TASK_TITLE);
	});

	it("drops tasks without a title", () => {
		const specs = tasksToContextSpecs(
			[buildTaskWithText({ title: "   " })],
			PREAMBLE,
			CONNECTION_ID,
		);

		expect(specs).toHaveLength(0);
	});

	it("maps several tasks into several specs", () => {
		const tasks = [
			buildTaskWithText({ id: "a", title: "First task" }),
			buildTaskWithText({ id: "b", title: "Second task" }),
		];

		const specs = tasksToContextSpecs(tasks, PREAMBLE, CONNECTION_ID);

		expect(specs.map((spec) => spec.contextName)).toEqual(["First task", "Second task"]);
	});

	it("exposes a non-empty default preamble", () => {
		expect(DEFAULT_TASKS_PREAMBLE.trim().length).toBeGreaterThan(0);
	});
});
