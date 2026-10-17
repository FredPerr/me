import { describe, expect, it } from "vitest";
import type { AgentRunStateView, AgentRunView } from "@/bindings";
import { buildAgentRunConversation, type LiveAgentRun } from "./agentRunConversation";

const CONTEXT_ID = "context-1";
const PAST_RUN_ID = "run-1";
const LIVE_RUN_ID = "run-2";
const PAST_PROMPT = "Fix the login bug";
const LIVE_PROMPT = "Add a logout button";
const FIRST_LINE = '{"method":"session/update"}';
const SECOND_LINE = '{"method":"session/update","done":true}';
const STARTED_AT_ISO = "2024-01-02T03:04:05.000Z";

function buildPastRun(overrides?: Partial<AgentRunView>): AgentRunView {
	return {
		id: PAST_RUN_ID,
		contextId: CONTEXT_ID,
		prompt: PAST_PROMPT,
		outputLines: [FIRST_LINE],
		state: "finished",
		exitCode: 0,
		trimmed: false,
		startedAt: STARTED_AT_ISO,
		...overrides,
	};
}

function buildLiveRun(overrides?: Partial<LiveAgentRun>): LiveAgentRun {
	return {
		runId: LIVE_RUN_ID,
		contextId: CONTEXT_ID,
		prompt: LIVE_PROMPT,
		lines: [FIRST_LINE, SECOND_LINE],
		state: "running",
		startedAt: Date.parse(STARTED_AT_ISO),
		...overrides,
	};
}

describe("buildAgentRunConversation", () => {
	it("returns no conversation when there are no runs", () => {
		const conversation = buildAgentRunConversation(CONTEXT_ID, [], undefined);

		expect(conversation).toBeUndefined();
	});

	it("builds a turn from a recorded run with its raw output lines as stdout", () => {
		const conversation = buildAgentRunConversation(CONTEXT_ID, [buildPastRun()], undefined);

		const [turn] = conversation?.turns ?? [];
		expect(turn.prompt).toBe(PAST_PROMPT);
		expect(turn.status).toBe("completed");
		expect(turn.lines).toEqual([{ id: 0, stream: "stdout", text: FIRST_LINE }]);
	});

	it("appends the live run after the recorded runs, oldest first", () => {
		const conversation = buildAgentRunConversation(CONTEXT_ID, [buildPastRun()], buildLiveRun());

		const prompts = conversation?.turns.map((turn) => turn.prompt);
		expect(prompts).toEqual([PAST_PROMPT, LIVE_PROMPT]);
	});

	it("shows the live run once when the recorded list already includes it", () => {
		const recordedLiveRun = buildPastRun({ id: LIVE_RUN_ID, prompt: LIVE_PROMPT });

		const conversation = buildAgentRunConversation(
			CONTEXT_ID,
			[buildPastRun(), recordedLiveRun],
			buildLiveRun(),
		);

		const liveTurns = conversation?.turns.filter((turn) => turn.sessionId === LIVE_RUN_ID) ?? [];
		expect(liveTurns).toHaveLength(1);
		expect(liveTurns[0].status).toBe("running");
	});

	it.each<[AgentRunStateView, string]>([
		["finished", "completed"],
		["failed", "failed"],
		["stopped", "killed"],
		["interrupted", "killed"],
	])("maps the %s run state to the %s display status", (state, expected) => {
		const conversation = buildAgentRunConversation(
			CONTEXT_ID,
			[buildPastRun({ state })],
			undefined,
		);

		expect(conversation?.turns[0].status).toBe(expected);
	});
});
