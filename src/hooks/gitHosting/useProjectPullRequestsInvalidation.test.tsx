// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectChanged } from "@/bindings";
import { projectPullRequestsQueryKey } from "./useProjectPullRequests";
import { useProjectPullRequestsInvalidation } from "./useProjectPullRequestsInvalidation";

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

const CHANGED_PROJECT_TAG = "acme";
const OTHER_PROJECT_TAG = "other";

function announceProjectChanged(tag: string) {
	for (const listener of projectChangedEvents.listeners) listener({ payload: { tag } });
}

function isInvalidated(queryClient: QueryClient, projectTag: string): boolean {
	return queryClient.getQueryState(projectPullRequestsQueryKey(projectTag))?.isInvalidated ?? false;
}

describe("useProjectPullRequestsInvalidation", () => {
	let queryClient: QueryClient;

	async function renderListening() {
		const wrapper = ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
		);
		const rendered = renderHook(() => useProjectPullRequestsInvalidation(), { wrapper });
		await waitFor(() => expect(projectChangedEvents.listeners).toHaveLength(1));
		return rendered;
	}

	beforeEach(() => {
		projectChangedEvents.listeners.length = 0;
		projectChangedEvents.unlisten.mockClear();
		queryClient = new QueryClient();
		for (const tag of [CHANGED_PROJECT_TAG, OTHER_PROJECT_TAG]) {
			queryClient.setQueryData(projectPullRequestsQueryKey(tag), []);
		}
	});

	it("refetches the pull requests of the changed project", async () => {
		await renderListening();

		announceProjectChanged(CHANGED_PROJECT_TAG);

		expect(isInvalidated(queryClient, CHANGED_PROJECT_TAG)).toBe(true);
	});

	it("leaves the pull requests of other projects as they are", async () => {
		await renderListening();

		announceProjectChanged(CHANGED_PROJECT_TAG);

		expect(isInvalidated(queryClient, OTHER_PROJECT_TAG)).toBe(false);
	});

	it("stops listening once unmounted", async () => {
		const { unmount } = await renderListening();

		unmount();

		expect(projectChangedEvents.unlisten).toHaveBeenCalledOnce();
	});
});
