import { describe, expect, it } from "vitest";
import { toWorkItemId, toWorkProjectId } from "@/domain/work-tracking/identifiers";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import { WorkItemPriority } from "@/domain/work-tracking/WorkItemPriority";
import { WorkItemStatus } from "@/domain/work-tracking/WorkItemStatus";
import { BranchNamingPolicy } from "@/models/BranchNaming";
import {
	buildTaskPreprompt,
	DEFAULT_TASKS_PREAMBLE,
	tasksToContextSpecs,
} from "./tasksToContextSpecs";

const TASK_TITLE = "Build the login page";
const TASK_DESCRIPTION = "Add email and password fields with validation.";
const TASK_URL = "https://example.teamwork.com/tasks/42";
const PREAMBLE = "Do the work described below.";
const FEATURE_PREFIX = "feature";

function buildTask(overrides?: Partial<WorkItem>): WorkItem {
	return {
		id: toWorkItemId("task-1"),
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

function policyWithPrefix(): BranchNamingPolicy {
	// allowNoPrefix=false forces the default prefix onto every branch name.
	return BranchNamingPolicy.create([FEATURE_PREFIX], false);
}

function policyWithoutPrefix(): BranchNamingPolicy {
	return BranchNamingPolicy.create([], true);
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
		const specs = tasksToContextSpecs([buildTask()], PREAMBLE, policyWithoutPrefix());

		expect(specs).toHaveLength(1);
		expect(specs[0].contextName).toBe(TASK_TITLE);
	});

	it("slugifies the title into the branch name using the naming policy", () => {
		const specs = tasksToContextSpecs([buildTask()], PREAMBLE, policyWithPrefix());

		expect(specs[0].branchName).toBe("feature/build-the-login-page");
	});

	it("stores the composed preamble and task text as the preprompt", () => {
		const specs = tasksToContextSpecs([buildTask()], PREAMBLE, policyWithoutPrefix());

		expect(specs[0].preprompt).toContain(PREAMBLE);
		expect(specs[0].preprompt).toContain(TASK_TITLE);
	});

	it("drops tasks whose title slugifies to an empty branch name", () => {
		const specs = tasksToContextSpecs(
			[buildTask({ title: "   " }), buildTask({ title: "###" })],
			PREAMBLE,
			policyWithoutPrefix(),
		);

		expect(specs).toHaveLength(0);
	});

	it("maps several tasks into several specs", () => {
		const tasks = [
			buildTask({ id: toWorkItemId("a"), title: "First task" }),
			buildTask({ id: toWorkItemId("b"), title: "Second task" }),
		];

		const specs = tasksToContextSpecs(tasks, PREAMBLE, policyWithoutPrefix());

		expect(specs.map((spec) => spec.contextName)).toEqual(["First task", "Second task"]);
	});

	it("exposes a non-empty default preamble", () => {
		expect(DEFAULT_TASKS_PREAMBLE.trim().length).toBeGreaterThan(0);
	});
});
