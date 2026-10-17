import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Page, Task } from "@/bindings";
import { ProviderKind } from "@/domain/work-tracking/ProviderKind";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import {
	buildConnection,
	buildTask,
	DEFAULT_CONNECTION_ID,
} from "@/test-utils/workTrackingBuilders";
import { listLinkedProjectTasks, MAX_PAGES_PER_PROJECT } from "./listLinkedProjectTasks";

const REMOTE_PROJECT_ID = "42";
const OTHER_REMOTE_PROJECT_ID = "43";
const NEXT_CURSOR = "2";
const LINKED_AT = new Date("2026-01-01T00:00:00.000Z");

const listProjectItems = vi.fn();
vi.mock("@/bindings", () => ({
	commands: {
		workTrackingListProjectItems: (
			connectionId: string,
			projectId: string,
			cursor: string | null,
		) => listProjectItems(connectionId, projectId, cursor),
	},
}));

function buildLink(remoteProjectId = REMOTE_PROJECT_ID): RemoteProjectLink {
	return RemoteProjectLink.create(
		{
			providerKind: ProviderKind.Teamwork,
			connectionId: DEFAULT_CONNECTION_ID,
			remoteProjectId,
			remoteProjectName: "Website",
		},
		LINKED_AT,
	);
}

function pageOf(items: Task[], nextCursor: string | null = null): Page<Task> {
	return { items, nextCursor };
}

describe("listLinkedProjectTasks", () => {
	beforeEach(() => {
		listProjectItems.mockReset();
		listProjectItems.mockResolvedValue(pageOf([buildTask()]));
	});

	it("returns the open tasks of every linked project", async () => {
		const tasks = await listLinkedProjectTasks(
			[buildLink(), buildLink(OTHER_REMOTE_PROJECT_ID)],
			[buildConnection()],
		);

		expect(tasks).toHaveLength(2);
		expect(tasks[0].connectionId).toBe(DEFAULT_CONNECTION_ID);
	});

	it("follows cursors until the last page", async () => {
		const secondTask = buildTask({ id: "2" });
		listProjectItems
			.mockResolvedValueOnce(pageOf([buildTask()], NEXT_CURSOR))
			.mockResolvedValueOnce(pageOf([secondTask]));

		const tasks = await listLinkedProjectTasks([buildLink()], [buildConnection()]);

		expect(tasks.map(({ task }) => task.id)).toEqual([buildTask().id, secondTask.id]);
		expect(listProjectItems).toHaveBeenLastCalledWith(
			DEFAULT_CONNECTION_ID,
			REMOTE_PROJECT_ID,
			NEXT_CURSOR,
		);
	});

	it("stops after the page limit", async () => {
		listProjectItems.mockResolvedValue(pageOf([buildTask()], NEXT_CURSOR));

		await listLinkedProjectTasks([buildLink()], [buildConnection()]);

		expect(listProjectItems).toHaveBeenCalledTimes(MAX_PAGES_PER_PROJECT);
	});

	it("drops done and cancelled tasks", async () => {
		const openTask = buildTask({ status: "inProgress" });
		listProjectItems.mockResolvedValue(
			pageOf([openTask, buildTask({ status: "done" }), buildTask({ status: "cancelled" })]),
		);

		const tasks = await listLinkedProjectTasks([buildLink()], [buildConnection()]);

		expect(tasks.map(({ task }) => task)).toEqual([openTask]);
	});

	it("skips links whose connection has no credential", async () => {
		const tasks = await listLinkedProjectTasks(
			[buildLink()],
			[buildConnection({ credentialConfigured: false })],
		);

		expect(tasks).toEqual([]);
	});
});
