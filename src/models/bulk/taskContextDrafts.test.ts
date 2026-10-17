import { describe, expect, it } from "vitest";
import { buildContext } from "@/test-utils/workspaceBuilders";
import { buildLinkedTask, DEFAULT_CONNECTION_ID } from "@/test-utils/workTrackingBuilders";
import {
	draftTaskContext,
	type TaskContextDraft,
	toBulkContextSpec,
	validateTaskContextDrafts,
} from "./taskContextDrafts";

const LOGIN_TITLE = "Add login page";
const BILLING_TITLE = "Fix billing export";
const BLANK_NAME = "   ";
const PREAMBLE = "Do the work described below.";

function draftNamed(id: string, contextName: string): TaskContextDraft {
	return {
		...draftTaskContext(buildLinkedTask({ id, title: LOGIN_TITLE }), PREAMBLE),
		contextName,
	};
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
		const draft = draftTaskContext(buildLinkedTask({ id: "1", title: LOGIN_TITLE }), PREAMBLE);

		const spec = toBulkContextSpec({ draft });

		expect(spec.task).toEqual({ connectionId: DEFAULT_CONNECTION_ID, taskId: "1" });
		expect(spec.agentPrompt).toContain(PREAMBLE);
	});
});
