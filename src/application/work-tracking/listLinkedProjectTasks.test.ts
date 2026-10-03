import { beforeEach, describe, expect, it, vi } from "vitest";
import { toWorkItemId, toWorkProjectId } from "@/domain/work-tracking/identifiers";
import type { Page } from "@/domain/work-tracking/Page";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { ProviderKind } from "@/domain/work-tracking/ProviderKind";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import { WorkItemPriority } from "@/domain/work-tracking/WorkItemPriority";
import { WorkItemStatus } from "@/domain/work-tracking/WorkItemStatus";
import { listLinkedProjectTasks, MAX_PAGES_PER_PROJECT } from "./listLinkedProjectTasks";

const CONNECTION_ID = "teamwork:acme.teamwork.com";
const REMOTE_PROJECT_ID = "42";
const OTHER_REMOTE_PROJECT_ID = "43";
const NEXT_CURSOR = "2";
const LINKED_AT = new Date("2026-01-01T00:00:00.000Z");

function buildLink(remoteProjectId = REMOTE_PROJECT_ID): RemoteProjectLink {
	return RemoteProjectLink.create(
		{
			providerKind: ProviderKind.Teamwork,
			connectionId: CONNECTION_ID,
			remoteProjectId,
			remoteProjectName: "Website",
		},
		LINKED_AT,
	);
}

function buildConnection(overrides?: Partial<ProviderConnection>): ProviderConnection {
	return {
		id: buildLink().connectionId,
		kind: ProviderKind.Teamwork,
		baseUrl: "https://acme.teamwork.com",
		displayName: "acme.teamwork.com",
		credentialConfigured: true,
		...overrides,
	};
}

function buildTask(overrides?: Partial<WorkItem>): WorkItem {
	return {
		id: toWorkItemId("1"),
		projectId: toWorkProjectId(REMOTE_PROJECT_ID),
		title: "Add login",
		status: WorkItemStatus.Todo,
		rawStatus: "new",
		priority: WorkItemPriority.None,
		assignees: [],
		labels: [],
		url: "https://acme.teamwork.com/app/tasks/1",
		...overrides,
	};
}

function pageOf(items: WorkItem[], nextCursor?: string): Page<WorkItem> {
	return { items, nextCursor };
}

describe("listLinkedProjectTasks", () => {
	const gateway = { listProjectItems: vi.fn() };

	beforeEach(() => {
		gateway.listProjectItems.mockReset();
		gateway.listProjectItems.mockResolvedValue(pageOf([buildTask()]));
	});

	it("returns the open tasks of every linked project", async () => {
		const tasks = await listLinkedProjectTasks(
			gateway,
			[buildLink(), buildLink(OTHER_REMOTE_PROJECT_ID)],
			[buildConnection()],
		);

		expect(tasks).toHaveLength(2);
		expect(tasks[0].connectionId).toBe(CONNECTION_ID);
	});

	it("follows cursors until the last page", async () => {
		const secondTask = buildTask({ id: toWorkItemId("2") });
		gateway.listProjectItems
			.mockResolvedValueOnce(pageOf([buildTask()], NEXT_CURSOR))
			.mockResolvedValueOnce(pageOf([secondTask]));

		const tasks = await listLinkedProjectTasks(gateway, [buildLink()], [buildConnection()]);

		expect(tasks.map(({ item }) => item.id)).toEqual([buildTask().id, secondTask.id]);
		expect(gateway.listProjectItems).toHaveBeenLastCalledWith(
			CONNECTION_ID,
			REMOTE_PROJECT_ID,
			NEXT_CURSOR,
		);
	});

	it("stops after the page limit", async () => {
		gateway.listProjectItems.mockResolvedValue(pageOf([buildTask()], NEXT_CURSOR));

		await listLinkedProjectTasks(gateway, [buildLink()], [buildConnection()]);

		expect(gateway.listProjectItems).toHaveBeenCalledTimes(MAX_PAGES_PER_PROJECT);
	});

	it("drops done and cancelled tasks", async () => {
		const openTask = buildTask({ status: WorkItemStatus.InProgress });
		gateway.listProjectItems.mockResolvedValue(
			pageOf([
				openTask,
				buildTask({ status: WorkItemStatus.Done }),
				buildTask({ status: WorkItemStatus.Cancelled }),
			]),
		);

		const tasks = await listLinkedProjectTasks(gateway, [buildLink()], [buildConnection()]);

		expect(tasks.map(({ item }) => item)).toEqual([openTask]);
	});

	it("skips links whose connection has no credential", async () => {
		const tasks = await listLinkedProjectTasks(
			gateway,
			[buildLink()],
			[buildConnection({ credentialConfigured: false })],
		);

		expect(tasks).toEqual([]);
	});
});
