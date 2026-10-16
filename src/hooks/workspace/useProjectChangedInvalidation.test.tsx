// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectChanged } from "@/bindings";
import { buildProject, PROJECT_TAG } from "@/test-utils/workspaceBuilders";
import { useProjectChangedInvalidation } from "./useProjectChangedInvalidation";
import { PROJECT_LIST_QUERY_KEY, projectQueryKey } from "./workspaceQueryKeys";

type ProjectChangedListener = (event: { payload: ProjectChanged }) => void;

const projectChangedEvents = vi.hoisted(() => ({
	listeners: [] as ProjectChangedListener[],
	unlisten: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	events: {
		projectChanged: {
			listen: async (listener: ProjectChangedListener) => {
				projectChangedEvents.listeners.push(listener);
				return projectChangedEvents.unlisten;
			},
		},
	},
}));

const OTHER_PROJECT_TAG = "billing";

function announceProjectChanged(tag: string) {
	for (const listener of projectChangedEvents.listeners) listener({ payload: { tag } });
}

function isInvalidated(queryClient: QueryClient, queryKey: readonly unknown[]): boolean {
	return queryClient.getQueryState(queryKey)?.isInvalidated ?? false;
}

describe("useProjectChangedInvalidation", () => {
	let queryClient: QueryClient;

	function renderInvalidation() {
		const wrapper = ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
		);
		return renderHook(() => useProjectChangedInvalidation(), { wrapper });
	}

	beforeEach(() => {
		projectChangedEvents.listeners.length = 0;
		projectChangedEvents.unlisten.mockClear();
		queryClient = new QueryClient();
		queryClient.setQueryData(PROJECT_LIST_QUERY_KEY, { projects: [] });
		queryClient.setQueryData(projectQueryKey(PROJECT_TAG), buildProject());
		queryClient.setQueryData(
			projectQueryKey(OTHER_PROJECT_TAG),
			buildProject({ tag: OTHER_PROJECT_TAG }),
		);
	});

	it("refetches the changed project and the project list", async () => {
		renderInvalidation();
		await waitFor(() => expect(projectChangedEvents.listeners).toHaveLength(1));

		announceProjectChanged(PROJECT_TAG);

		expect(isInvalidated(queryClient, projectQueryKey(PROJECT_TAG))).toBe(true);
		expect(isInvalidated(queryClient, PROJECT_LIST_QUERY_KEY)).toBe(true);
	});

	it("leaves the other projects as they are", async () => {
		renderInvalidation();
		await waitFor(() => expect(projectChangedEvents.listeners).toHaveLength(1));

		announceProjectChanged(PROJECT_TAG);

		expect(isInvalidated(queryClient, projectQueryKey(OTHER_PROJECT_TAG))).toBe(false);
	});

	it("stops listening once unmounted", async () => {
		const { unmount } = renderInvalidation();
		await waitFor(() => expect(projectChangedEvents.listeners).toHaveLength(1));

		unmount();

		expect(projectChangedEvents.unlisten).toHaveBeenCalledOnce();
	});
});
