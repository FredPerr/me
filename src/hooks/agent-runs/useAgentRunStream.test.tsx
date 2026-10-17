// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentRunChanged } from "@/bindings";
import { contextAgentRunsQueryKey } from "./agentRunQueryKeys";
import { useAgentRunStream } from "./useAgentRunStream";

type AgentRunChangedListener = (event: { payload: AgentRunChanged }) => void;

const agentRunChangedEvents = vi.hoisted(() => ({
	listeners: [] as AgentRunChangedListener[],
	unlisten: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	events: {
		agentRunChanged: {
			listen: async (listener: AgentRunChangedListener) => {
				agentRunChangedEvents.listeners.push(listener);
				return agentRunChangedEvents.unlisten;
			},
		},
	},
}));

const CONTEXT_ID = "context-1";
const RUN_ID = "run-1";
const FIRST_LINE = "hello";
const SECOND_LINE = "world";

function emit(update: AgentRunChanged["update"], runId = RUN_ID, contextId = CONTEXT_ID) {
	act(() => {
		for (const listener of agentRunChangedEvents.listeners) {
			listener({ payload: { contextId, runId, update } });
		}
	});
}

describe("useAgentRunStream", () => {
	let queryClient: QueryClient;

	async function renderStream() {
		const wrapper = ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
		);
		const rendered = renderHook(() => useAgentRunStream(), { wrapper });
		await waitFor(() => expect(agentRunChangedEvents.listeners).toHaveLength(1));
		return rendered;
	}

	beforeEach(() => {
		agentRunChangedEvents.listeners.length = 0;
		agentRunChangedEvents.unlisten.mockClear();
		queryClient = new QueryClient();
	});

	it("marks a context running once its run starts", async () => {
		const { result } = await renderStream();

		emit({ kind: "started" });

		expect(result.current.isRunning(CONTEXT_ID)).toBe(true);
	});

	it("appends a line update to the live run transcript", async () => {
		const { result } = await renderStream();

		emit({ kind: "started" });
		emit({ kind: "line", text: FIRST_LINE });
		emit({ kind: "line", text: SECOND_LINE });

		expect(result.current.liveRunFor(CONTEXT_ID)?.lines).toEqual([FIRST_LINE, SECOND_LINE]);
	});

	it("shows the noted prompt on the live run the start event omits", async () => {
		const notedPrompt = "Investigate the flaky test";
		const { result } = await renderStream();

		act(() => result.current.notePendingPrompt(CONTEXT_ID, notedPrompt));
		emit({ kind: "started" });

		expect(result.current.liveRunFor(CONTEXT_ID)?.prompt).toBe(notedPrompt);
	});

	it("flips running off when the run ends", async () => {
		const { result } = await renderStream();

		emit({ kind: "started" });
		emit({ kind: "ended", finalState: "finished", exitCode: 0 });

		expect(result.current.isRunning(CONTEXT_ID)).toBe(false);
	});

	it("refetches the context's recorded runs when its run ends", async () => {
		queryClient.setQueryData(contextAgentRunsQueryKey(CONTEXT_ID), []);
		await renderStream();

		emit({ kind: "started" });
		emit({ kind: "ended", finalState: "finished", exitCode: 0 });

		expect(queryClient.getQueryState(contextAgentRunsQueryKey(CONTEXT_ID))?.isInvalidated).toBe(
			true,
		);
	});

	it("stops listening once unmounted", async () => {
		const { unmount } = await renderStream();

		unmount();

		expect(agentRunChangedEvents.unlisten).toHaveBeenCalledOnce();
	});
});
