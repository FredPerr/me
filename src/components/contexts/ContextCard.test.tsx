// @vitest-environment jsdom
import { MantineProvider } from "@mantine/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { ContextOriginView, MatchedPullRequestView } from "@/bindings";
import "@/i18n";
import {
	API_REPOSITORY_ID,
	buildContext,
	buildContextBranch,
	buildProject,
} from "@/test-utils/workspaceBuilders";
import { ContextCard } from "./ContextCard";

const openUrlMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/plugin-opener", () => ({
	openUrl: openUrlMock,
}));

vi.mock("@/hooks/useOpenInIde", () => ({
	useOpenInIde: () => ({ open: vi.fn(), isAvailable: false }),
}));

vi.mock("@/hooks/useActiveWorkspaces", () => ({
	useActiveWorkspaces: () => ({ isActive: () => false }),
}));

vi.mock("@/hooks/useContextDiffStats", () => ({
	useContextDiffStats: () => ({ statsByRepository: {}, hasBaseContext: true }),
}));

vi.mock("@/hooks/workspace/useContextMutations", async (importOriginal) => {
	const actual = await importOriginal<typeof import("@/hooks/workspace/useContextMutations")>();
	return {
		...actual,
		useDeleteContext: () => ({ mutate: vi.fn(), isPending: false }),
		useSetContextStatic: () => ({ mutate: vi.fn() }),
	};
});

const MATCHED_PR_NUMBER = 12;
const MATCHED_PR_URL = "https://github.com/acme/api/pull/12";
const FROM_PULL_REQUESTS: ContextOriginView = { kind: "fromPullRequests" };

function buildMatchedPullRequest(
	overrides?: Partial<MatchedPullRequestView>,
): MatchedPullRequestView {
	return {
		host: "github",
		number: MATCHED_PR_NUMBER,
		url: MATCHED_PR_URL,
		...overrides,
	};
}

beforeAll(() => {
	window.matchMedia ??= (query: string) => ({
		matches: false,
		media: query,
		onchange: null,
		addListener: () => {},
		removeListener: () => {},
		addEventListener: () => {},
		removeEventListener: () => {},
		dispatchEvent: () => false,
	});
	globalThis.ResizeObserver ??= class {
		observe() {}
		unobserve() {}
		disconnect() {}
	};
});

describe("ContextCard", () => {
	const project = buildProject();

	function renderCard(context: ReturnType<typeof buildContext>) {
		const queryClient = new QueryClient({
			defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
		});
		const wrapper = ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={queryClient}>
				<MantineProvider env="test">{children}</MantineProvider>
			</QueryClientProvider>
		);
		render(<ContextCard context={context} project={project} />, { wrapper });
	}

	beforeEach(() => {
		openUrlMock.mockReset();
	});

	afterEach(cleanup);

	it("shows the pull-request origin icon when the context was built from pull requests", () => {
		const context = buildContext({ origin: FROM_PULL_REQUESTS });

		renderCard(context);

		expect(screen.getByLabelText("Created from pull requests")).toBeDefined();
	});

	it("shows the recorded pull-request number linking to the pull request", async () => {
		const context = buildContext({
			branches: [
				buildContextBranch({
					repositoryId: API_REPOSITORY_ID,
					pullRequestMatched: buildMatchedPullRequest(),
				}),
			],
		});

		renderCard(context);

		await userEvent.click(screen.getByText(`#${MATCHED_PR_NUMBER}`));

		expect(openUrlMock).toHaveBeenCalledWith(MATCHED_PR_URL);
	});

	it("shows the origin icon when a branch has a recorded match but the origin is not fromPullRequests", () => {
		const context = buildContext({
			origin: { kind: "manual" },
			branches: [buildContextBranch({ pullRequestMatched: buildMatchedPullRequest() })],
		});

		renderCard(context);

		expect(screen.getByLabelText("Created from pull requests")).toBeDefined();
	});

	it("shows neither the origin icon nor a recorded number for a plain context", () => {
		const context = buildContext({
			origin: { kind: "manual" },
			branches: [buildContextBranch({ pullRequestMatched: null })],
		});

		renderCard(context);

		expect(screen.queryByLabelText("Created from pull requests")).toBeNull();
		expect(screen.queryByText(`#${MATCHED_PR_NUMBER}`)).toBeNull();
	});
});
