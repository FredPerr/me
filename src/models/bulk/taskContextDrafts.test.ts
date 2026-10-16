import { describe, expect, it } from "vitest";
import type { LinkedTask } from "@/application/work-tracking/listLinkedProjectTasks";
import { toWorkItemId, toWorkProjectId } from "@/domain/work-tracking/identifiers";
import { ProviderKind } from "@/domain/work-tracking/ProviderKind";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import { WorkItemPriority } from "@/domain/work-tracking/WorkItemPriority";
import { WorkItemStatus } from "@/domain/work-tracking/WorkItemStatus";
import { buildContext } from "@/test-utils/workspaceBuilders";
import {
	draftTaskContext,
	type TaskContextDraft,
	toBulkContextSpec,
	validateTaskContextDrafts,
} from "./taskContextDrafts";

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
const BLANK_NAME = "   ";
const PREAMBLE = "Do the work described below.";

function buildTask(id: string, title: string): LinkedTask {
	return {
		connectionId: CONNECTION_ID,
		item: {
			id: toWorkItemId(id),
			projectId: toWorkProjectId("42"),
			title,
			status: WorkItemStatus.Todo,
			rawStatus: "new",
			priority: WorkItemPriority.None,
			assignees: [],
			labels: [],
			url: `https://acme.teamwork.com/app/tasks/${id}`,
		},
	};
}

function draftNamed(id: string, contextName: string): TaskContextDraft {
	return { ...draftTaskContext(buildTask(id, LOGIN_TITLE), PREAMBLE), contextName };
}

describe("validateTaskContextDrafts", () => {
	it("accepts drafts with distinct new names", () => {
		const drafts = [draftNamed("1", LOGIN_TITLE), draftNamed("2", BILLING_TITLE)];

		const validated = validateTaskContextDrafts(drafts, []);

		expect(validated.map(({ issue }) => issue)).toEqual([undefined, undefined]);
	});

	it("requires a name", () => {
		const [validated] = validateTaskContextDrafts([draftNamed("1", BLANK_NAME)], []);

		expect(validated.issue).toBe("nameRequired");
	});

	it("flags a name used twice in the batch, ignoring case", () => {
		const drafts = [draftNamed("1", LOGIN_TITLE), draftNamed("2", LOGIN_TITLE.toUpperCase())];

		const validated = validateTaskContextDrafts(drafts, []);

		expect(validated.map(({ issue }) => issue)).toEqual(["duplicateName", "duplicateName"]);
	});

	it("flags a name an existing context already uses", () => {
		const [validated] = validateTaskContextDrafts(
			[draftNamed("1", LOGIN_TITLE)],
			[buildContext({ name: LOGIN_TITLE })],
		);

		expect(validated.issue).toBe("duplicateName");
	});
});

describe("toBulkContextSpec", () => {
	it("creates the context from its task with the draft prompt", () => {
		const draft = draftTaskContext(buildTask("1", LOGIN_TITLE), PREAMBLE);

		const spec = toBulkContextSpec({ draft });

		expect(spec.task).toEqual({ connectionId: CONNECTION_ID, taskId: "1" });
		expect(spec.agentPrompt).toContain(PREAMBLE);
	});
});
