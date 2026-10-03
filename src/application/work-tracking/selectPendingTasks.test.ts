import { describe, expect, it } from "vitest";
import { toWorkItemId, toWorkProjectId } from "@/domain/work-tracking/identifiers";
import { ProviderKind } from "@/domain/work-tracking/ProviderKind";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import { WorkItemPriority } from "@/domain/work-tracking/WorkItemPriority";
import { WorkItemStatus } from "@/domain/work-tracking/WorkItemStatus";
import { Context } from "@/models/Project";
import type { LinkedTask } from "./listLinkedProjectTasks";
import { selectPendingTasks, workItemRefOf } from "./selectPendingTasks";

const CONNECTION_ID = RemoteProjectLink.create(
	{
		providerKind: ProviderKind.Teamwork,
		connectionId: "teamwork:acme.teamwork.com",
		remoteProjectId: "42",
		remoteProjectName: "Website",
	},
	new Date(),
).connectionId;
const LOGIN_TITLE = "Add login page";
const BILLING_TITLE = "Fix billing export";

function buildTask(id: string, title: string): LinkedTask {
	const item: WorkItem = {
		id: toWorkItemId(id),
		projectId: toWorkProjectId("42"),
		title,
		status: WorkItemStatus.Todo,
		rawStatus: "new",
		priority: WorkItemPriority.None,
		assignees: [],
		labels: [],
		url: `https://acme.teamwork.com/app/tasks/${id}`,
	};
	return { connectionId: CONNECTION_ID, item };
}

function buildContextFrom(task: LinkedTask): Context {
	return new Context(
		"context-1",
		task.item.title,
		[],
		false,
		undefined,
		{},
		undefined,
		undefined,
		false,
		workItemRefOf(task),
	);
}

describe("selectPendingTasks", () => {
	const loginTask = buildTask("1", LOGIN_TITLE);
	const billingTask = buildTask("2", BILLING_TITLE);

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
