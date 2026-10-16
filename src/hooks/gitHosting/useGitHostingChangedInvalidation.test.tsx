// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { GitHostingChanged } from "@/bindings";
import { PROJECT_LIST_QUERY_KEY } from "@/hooks/workspace/workspaceQueryKeys";
import { HOSTED_ACCOUNTS_QUERY_KEY } from "./gitHostingQueryKeys";
import { useGitHostingChangedInvalidation } from "./useGitHostingChangedInvalidation";

type GitHostingChangedListener = (event: { payload: GitHostingChanged }) => void;

const gitHostingChangedEvents = vi.hoisted(() => ({
	listeners: [] as GitHostingChangedListener[],
	unlisten: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	events: {
		gitHostingChanged: {
			listen: async (listener: GitHostingChangedListener) => {
				gitHostingChangedEvents.listeners.push(listener);
				return gitHostingChangedEvents.unlisten;
			},
		},
	},
}));

const CHANGED_HOST = "github";

function announceGitHostingChanged() {
	for (const listener of gitHostingChangedEvents.listeners)
		listener({ payload: { host: CHANGED_HOST } });
}

function isInvalidated(queryClient: QueryClient, queryKey: readonly unknown[]): boolean {
	return queryClient.getQueryState(queryKey)?.isInvalidated ?? false;
}

describe("useGitHostingChangedInvalidation", () => {
	let queryClient: QueryClient;

	function renderInvalidation() {
		const wrapper = ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
		);
		return renderHook(() => useGitHostingChangedInvalidation(), { wrapper });
	}

	beforeEach(() => {
		gitHostingChangedEvents.listeners.length = 0;
		gitHostingChangedEvents.unlisten.mockClear();
		queryClient = new QueryClient();
		queryClient.setQueryData(HOSTED_ACCOUNTS_QUERY_KEY, []);
		queryClient.setQueryData(PROJECT_LIST_QUERY_KEY, { projects: [] });
	});

	it("refetches the Git hosting queries", async () => {
		renderInvalidation();
		await waitFor(() => expect(gitHostingChangedEvents.listeners).toHaveLength(1));

		announceGitHostingChanged();

		expect(isInvalidated(queryClient, HOSTED_ACCOUNTS_QUERY_KEY)).toBe(true);
	});

	it("leaves the other queries as they are", async () => {
		renderInvalidation();
		await waitFor(() => expect(gitHostingChangedEvents.listeners).toHaveLength(1));

		announceGitHostingChanged();

		expect(isInvalidated(queryClient, PROJECT_LIST_QUERY_KEY)).toBe(false);
	});

	it("stops listening once unmounted", async () => {
		const { unmount } = renderInvalidation();
		await waitFor(() => expect(gitHostingChangedEvents.listeners).toHaveLength(1));

		unmount();

		expect(gitHostingChangedEvents.unlisten).toHaveBeenCalledOnce();
	});
});
