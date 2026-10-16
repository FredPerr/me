// @vitest-environment jsdom
import { MantineProvider } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type {
	BranchPullRequestStatusView,
	BranchPullRequestView,
	CreatedPullRequestView,
	GitHostingCommandError,
	HostKindView,
} from "@/bindings";
import "@/i18n";
import {
	API_REPOSITORY_ID,
	buildContext,
	buildContextBranch,
	buildDefaultContext,
	buildProject,
	MAIN_BRANCH,
	PROJECT_PATH,
	PROJECT_TAG,
} from "@/test-utils/workspaceBuilders";
import { CreatePullRequestModal } from "./CreatePullRequestModal";

const gitHostingCommands = vi.hoisted(() => ({
	contextPullRequests: vi.fn(),
	createPullRequest: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	commands: {
		gitHostingContextPullRequests: gitHostingCommands.contextPullRequests,
		gitHostingCreatePullRequest: gitHostingCommands.createPullRequest,
	},
}));

// Generating a description runs an AI session; these tests never generate one.
vi.mock("@/hooks/usePullRequestDescriptionDraft", () => ({
	usePullRequestDescriptionDraft: () => ({
		stateFor: () => ({ status: "idle", description: "", error: null }),
		generate: vi.fn(),
	}),
}));

vi.mock("@uiw/react-md-editor", () => ({
	default: ({ value }: { value: string }) => <textarea readOnly value={value} />,
}));

const LOGIN_BRANCH = "feature/login";
const TITLE = "Add the login page";
const DESCRIPTION = "Adds a login page.";
const CREATED: CreatedPullRequestView = {
	number: 12,
	url: "https://github.com/acme/api/pull/12",
};
const PUSH_SKIPPED_ERROR: GitHostingCommandError = {
	kind: "pushSkipped",
	message: "The branch could not be pushed: it has no remote branch to push to.",
	retryAfterSeconds: null,
	status: null,
};

const project = buildProject({
	contexts: [buildDefaultContext([buildContextBranch()])],
});

const loginContext = buildContext({
	branches: [
		buildContextBranch({
			branch: LOGIN_BRANCH,
			worktreePath: `${PROJECT_PATH}/.worktrees/feature-login/api`,
			pullRequestDraft: { title: TITLE, description: DESCRIPTION },
		}),
	],
});

function buildLoginBranch(
	status: BranchPullRequestStatusView,
	host: HostKindView | null = "github",
): BranchPullRequestView {
	return { repositoryId: API_REPOSITORY_ID, branch: LOGIN_BRANCH, host, status };
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

describe("CreatePullRequestModal", () => {
	function renderModal() {
		const queryClient = new QueryClient({
			defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
		});
		const wrapper = ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={queryClient}>
				<MantineProvider env="test">
					<Notifications />
					{children}
				</MantineProvider>
			</QueryClientProvider>
		);
		render(
			<CreatePullRequestModal opened onClose={vi.fn()} project={project} context={loginContext} />,
			{ wrapper },
		);
	}

	function createButton() {
		return screen.getByRole("button", { name: "Create pull request" });
	}

	beforeEach(() => {
		gitHostingCommands.contextPullRequests
			.mockReset()
			.mockResolvedValue([buildLoginBranch({ kind: "ok", pullRequest: null })]);
		gitHostingCommands.createPullRequest.mockReset().mockResolvedValue(CREATED);
	});

	afterEach(cleanup);

	it("creates the pull request from the context branch into the base branch", async () => {
		renderModal();

		await userEvent.click(createButton());

		expect(gitHostingCommands.createPullRequest).toHaveBeenCalledWith(
			PROJECT_TAG,
			API_REPOSITORY_ID,
			{ title: TITLE, body: DESCRIPTION, head: LOGIN_BRANCH, base: MAIN_BRANCH, draft: false },
		);
	});

	it("links to the created pull request", async () => {
		renderModal();

		await userEvent.click(createButton());

		const link = await screen.findByRole("link", { name: `View PR #${CREATED.number}` });
		expect(link.getAttribute("href")).toBe(CREATED.url);
	});

	it("shows the message of a failed creation", async () => {
		gitHostingCommands.createPullRequest.mockRejectedValue(PUSH_SKIPPED_ERROR);
		renderModal();

		await userEvent.click(createButton());

		expect(await screen.findByText(PUSH_SKIPPED_ERROR.message)).toBeDefined();
	});

	it("asks to sign in to the Git host of a branch that is not connected", async () => {
		gitHostingCommands.contextPullRequests.mockResolvedValue([
			buildLoginBranch({ kind: "notConnected" }, "bitbucket"),
		]);
		renderModal();

		expect(
			await screen.findByText(
				"Sign in to Bitbucket in Settings → Integrations to create a pull request here.",
			),
		).toBeDefined();
		expect(createButton().hasAttribute("disabled")).toBe(true);
	});

	it("says when the repository's remote is not on a supported Git host", async () => {
		gitHostingCommands.contextPullRequests.mockResolvedValue([
			buildLoginBranch({ kind: "unsupportedHost" }, null),
		]);
		renderModal();

		expect(
			await screen.findByText(
				"This repository's remote is not on GitHub or Bitbucket, so a pull request cannot be created here.",
			),
		).toBeDefined();
	});
});
