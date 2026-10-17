import { describe, expect, it } from "vitest";
import type { ContextView } from "@/bindings";
import { buildContext } from "@/test-utils/workspaceBuilders";
import { buildLinkedTask } from "@/test-utils/workTrackingBuilders";
import type { LinkedTask } from "./listLinkedProjectTasks";
import { selectPendingTasks, taskRefOf } from "./selectPendingTasks";

const LOGIN_TITLE = "Add login page";
const BILLING_TITLE = "Fix billing export";

function buildContextFrom(task: LinkedTask): ContextView {
	return buildContext({ name: task.task.title, origin: { kind: "fromTask", ...taskRefOf(task) } });
}

describe("selectPendingTasks", () => {
	const loginTask = buildLinkedTask({ id: "1", title: LOGIN_TITLE });
	const billingTask = buildLinkedTask({ id: "2", title: BILLING_TITLE });

	it("keeps tasks that have no context yet", () => {
		expect(selectPendingTasks([loginTask, billingTask], [])).toEqual([loginTask, billingTask]);
	});

	it("hides tasks a context was created from", () => {
		const result = selectPendingTasks([loginTask, billingTask], [buildContextFrom(loginTask)]);

		expect(result).toEqual([billingTask]);
	});

	it("matches the search query against the task title", () => {
		const result = selectPendingTasks([loginTask, billingTask], [], "  BILLING ");

		expect(result).toEqual([billingTask]);
	});
});
