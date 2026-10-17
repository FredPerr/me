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
	ContextView,
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

const commandMocks = vi.hoisted(() => ({
	contextPullRequests: vi.fn(),
	createPullRequest: vi.fn(),
	suggestPullRequest: vi.fn(),
	cancelPullRequest: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	commands: {
		gitHostingContextPullRequests: commandMocks.contextPullRequests,
		gitHostingCreatePullRequest: commandMocks.createPullRequest,
		aiAssistSuggestPullRequest: commandMocks.suggestPullRequest,
		aiAssistCancelPullRequest: commandMocks.cancelPullRequest,
	},
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
	message: "The branch could not be pushed: the repository has no remote.",
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
		commandMocks.contextPullRequests
			.mockReset()
			.mockResolvedValue([buildLoginBranch({ kind: "ok", pullRequest: null })]);
		commandMocks.createPullRequest.mockReset().mockResolvedValue(CREATED);
		commandMocks.suggestPullRequest.mockReset();
		commandMocks.cancelPullRequest.mockReset();
	});

	afterEach(cleanup);

	it("creates the pull request from the context branch into the base branch", async () => {
		renderModal();

		await userEvent.click(createButton());

		expect(commandMocks.createPullRequest).toHaveBeenCalledWith(PROJECT_TAG, API_REPOSITORY_ID, {
			title: TITLE,
			body: DESCRIPTION,
			head: LOGIN_BRANCH,
			base: MAIN_BRANCH,
			draft: false,
		});
	});

	it("links to the created pull request", async () => {
		renderModal();

		await userEvent.click(createButton());

		const link = await screen.findByRole("link", { name: `View PR #${CREATED.number}` });
		expect(link.getAttribute("href")).toBe(CREATED.url);
	});

	it("shows the message of a failed creation", async () => {
		commandMocks.createPullRequest.mockRejectedValue(PUSH_SKIPPED_ERROR);
		renderModal();

		await userEvent.click(createButton());

		expect(await screen.findByText(PUSH_SKIPPED_ERROR.message)).toBeDefined();
	});

	it("asks to sign in to the Git host of a branch that is not connected", async () => {
		commandMocks.contextPullRequests.mockResolvedValue([
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
		commandMocks.contextPullRequests.mockResolvedValue([
			buildLoginBranch({ kind: "unsupportedHost" }, null),
		]);
		renderModal();

		expect(
			await screen.findByText(
				"This repository's remote is not on GitHub or Bitbucket, so a pull request cannot be created here.",
			),
		).toBeDefined();
	});

	const GENERATED_TITLE = "Generated title";
	const GENERATED_DESCRIPTION = "- Generated bullet";

	function renderWith(context: ContextView) {
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
			<CreatePullRequestModal opened onClose={vi.fn()} project={project} context={context} />,
			{ wrapper },
		);
	}

	function generateButton() {
		return screen.getByRole("button", { name: "Generate" });
	}

	it("fills the title and description when the user has not typed a title", async () => {
		commandMocks.suggestPullRequest.mockResolvedValue({
			title: GENERATED_TITLE,
			description: GENERATED_DESCRIPTION,
		});
		const untitled = buildContext({
			branches: [
				buildContextBranch({
					branch: LOGIN_BRANCH,
					worktreePath: `${PROJECT_PATH}/.worktrees/feature-login/api`,
					pullRequestDraft: { title: "", description: "" },
				}),
			],
		});
		renderWith(untitled);

		await userEvent.click(generateButton());

		expect(commandMocks.suggestPullRequest).toHaveBeenCalledWith(
			PROJECT_TAG,
			untitled.id,
			API_REPOSITORY_ID,
		);
		expect((await screen.findByDisplayValue(GENERATED_TITLE)) as HTMLInputElement).toBeDefined();
		expect(await screen.findByDisplayValue(GENERATED_DESCRIPTION)).toBeDefined();
	});

	it("keeps a title the user already typed and fills only the description", async () => {
		commandMocks.suggestPullRequest.mockResolvedValue({
			title: GENERATED_TITLE,
			description: GENERATED_DESCRIPTION,
		});
		renderWith(loginContext);

		await userEvent.click(generateButton());

		// The saved title is kept; only the description is replaced.
		expect(await screen.findByDisplayValue(TITLE)).toBeDefined();
		expect(await screen.findByDisplayValue(GENERATED_DESCRIPTION)).toBeDefined();
	});

	it("shows the message of a failed generation", async () => {
		commandMocks.suggestPullRequest.mockRejectedValue({
			kind: "emptyDiff",
			message: "No changes found against the base branch.",
		});
		renderWith(loginContext);

		await userEvent.click(generateButton());

		expect(await screen.findByText("No changes found against the base branch.")).toBeDefined();
	});
});
