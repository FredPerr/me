// @vitest-environment jsdom
import { MantineProvider } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type {
	BranchChoiceInput,
	CreateContextInput,
	HostedPullRequestView,
	ProjectView,
	RepositoryBranchInput,
	RepositoryPullRequestsView,
} from "@/bindings";
import "@/i18n";
import {
	API_REPOSITORY_ID,
	buildContextBranch,
	buildDefaultContext,
	buildProject,
	buildRepository,
	MAIN_BRANCH,
	WEB_REPOSITORY_ID,
} from "@/test-utils/workspaceBuilders";
import { CreateFromPullRequestsModal } from "./CreateFromPullRequestsModal";

const LOGIN_BRANCH = "feature/login";
const SUGGESTED_BRANCH = "user/feature/login";
const API_PULL_REQUEST_NUMBER = 12;
const WEB_PULL_REQUEST_NUMBER = 34;
const FORK_PULL_REQUEST_NUMBER = 99;
const FORK_BRANCH = "fork/patch";

const commandMocks = vi.hoisted(() => ({
	projectPullRequests: vi.fn(),
	suggestBranchName: vi.fn(),
	createContext: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	commands: {
		gitHostingProjectPullRequests: commandMocks.projectPullRequests,
		workspaceSuggestBranchName: commandMocks.suggestBranchName,
		workspaceCreateContext: commandMocks.createContext,
	},
}));

function buildPullRequest(overrides?: Partial<HostedPullRequestView>): HostedPullRequestView {
	return {
		number: API_PULL_REQUEST_NUMBER,
		title: "Add the login page",
		url: "https://github.com/acme/api/pull/12",
		author: "octocat",
		headBranch: LOGIN_BRANCH,
		baseBranch: MAIN_BRANCH,
		isFork: false,
		draft: false,
		state: "open",
		updatedAt: "2024-05-01T12:00:00Z",
		...overrides,
	};
}

function buildRepositoryPullRequests(
	repositoryId: string,
	repositoryName: string,
	pullRequests: HostedPullRequestView[],
): RepositoryPullRequestsView {
	return {
		repositoryId,
		repositoryName,
		host: "github",
		status: { kind: "ok", pullRequests, truncated: false },
	};
}

const API_REPOSITORY = buildRepository({ id: API_REPOSITORY_ID, name: "api" });
const WEB_REPOSITORY = buildRepository({
	id: WEB_REPOSITORY_ID,
	name: "web",
	relativePath: "./web",
});

const project: ProjectView = buildProject({
	repositories: [API_REPOSITORY, WEB_REPOSITORY],
	contexts: [
		buildDefaultContext([
			buildContextBranch({ repositoryId: API_REPOSITORY_ID, branch: MAIN_BRANCH }),
			buildContextBranch({ repositoryId: WEB_REPOSITORY_ID, branch: MAIN_BRANCH }),
		]),
	],
});

/** The project's two repositories each have the shared login branch open. */
function bothRepositoriesHaveLogin(): RepositoryPullRequestsView[] {
	return [
		buildRepositoryPullRequests(API_REPOSITORY_ID, "api", [
			buildPullRequest({ number: API_PULL_REQUEST_NUMBER }),
		]),
		buildRepositoryPullRequests(WEB_REPOSITORY_ID, "web", [
			buildPullRequest({
				number: WEB_PULL_REQUEST_NUMBER,
				url: "https://github.com/acme/web/pull/34",
			}),
		]),
	];
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

describe("CreateFromPullRequestsModal", () => {
	function renderModal(onClose = vi.fn()) {
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
		render(<CreateFromPullRequestsModal opened onClose={onClose} project={project} />, { wrapper });
		return { onClose };
	}

	function createButton() {
		return screen.getByRole("button", { name: "Create" });
	}

	/** The one branch input the create call carries for a repository. */
	function branchFor(input: CreateContextInput, repositoryId: string): BranchChoiceInput {
		const match = input.branches.find(
			(entry: RepositoryBranchInput) => entry.repositoryId === repositoryId,
		);
		if (!match) throw new Error(`no branch input for ${repositoryId}`);
		return match.branch;
	}

	beforeEach(() => {
		commandMocks.projectPullRequests.mockReset().mockResolvedValue(bothRepositoriesHaveLogin());
		commandMocks.suggestBranchName.mockReset().mockResolvedValue(SUGGESTED_BRANCH);
		commandMocks.createContext.mockReset().mockResolvedValue(project);
	});

	afterEach(cleanup);

	it("picks a group to select the matching pull request in each repository and builds their inputs", async () => {
		renderModal();

		const groupSelect = await screen.findByPlaceholderText("Pick a branch across repositories");
		await userEvent.click(groupSelect);
		await userEvent.click(
			await screen.findByRole("option", {
				name: `${LOGIN_BRANCH} — api #${API_PULL_REQUEST_NUMBER}, web #${WEB_PULL_REQUEST_NUMBER}`,
			}),
		);

		await userEvent.click(createButton());

		await waitFor(() => expect(commandMocks.createContext).toHaveBeenCalled());
		const input: CreateContextInput = commandMocks.createContext.mock.calls[0][0];
		expect(input.origin).toEqual({ kind: "fromPullRequests" });
		expect(input.status).toBe("review");
		expect(branchFor(input, API_REPOSITORY_ID)).toEqual({
			kind: "pullRequest",
			headBranch: LOGIN_BRANCH,
			baseBranch: MAIN_BRANCH,
			host: "github",
			number: API_PULL_REQUEST_NUMBER,
			url: "https://github.com/acme/api/pull/12",
		});
		expect(branchFor(input, WEB_REPOSITORY_ID)).toMatchObject({
			kind: "pullRequest",
			number: WEB_PULL_REQUEST_NUMBER,
		});
	});

	it("gives a new branch to a repository without a matching pull request", async () => {
		commandMocks.projectPullRequests.mockResolvedValue([
			buildRepositoryPullRequests(API_REPOSITORY_ID, "api", [buildPullRequest()]),
			buildRepositoryPullRequests(WEB_REPOSITORY_ID, "web", []),
		]);
		renderModal();

		const groupSelect = await screen.findByPlaceholderText("Pick a branch across repositories");
		await userEvent.click(groupSelect);
		await userEvent.click(await screen.findByRole("option", { name: new RegExp(LOGIN_BRANCH) }));

		await userEvent.click(createButton());

		await waitFor(() => expect(commandMocks.createContext).toHaveBeenCalled());
		const input: CreateContextInput = commandMocks.createContext.mock.calls[0][0];
		expect(branchFor(input, API_REPOSITORY_ID).kind).toBe("pullRequest");
		expect(branchFor(input, WEB_REPOSITORY_ID)).toEqual({
			kind: "newBranch",
			name: SUGGESTED_BRANCH,
			baseBranch: MAIN_BRANCH,
		});
	});

	it("disables a pull request from a fork", async () => {
		commandMocks.projectPullRequests.mockResolvedValue([
			buildRepositoryPullRequests(API_REPOSITORY_ID, "api", [
				buildPullRequest({
					number: FORK_PULL_REQUEST_NUMBER,
					headBranch: FORK_BRANCH,
					isFork: true,
				}),
			]),
			buildRepositoryPullRequests(WEB_REPOSITORY_ID, "web", []),
		]);
		renderModal();

		const repositorySelect = await screen.findByPlaceholderText("Select an open pull request");
		await userEvent.click(repositorySelect);

		const forkOption = await screen.findByRole("option", {
			name: `#${FORK_PULL_REQUEST_NUMBER} Add the login page (from a fork)`,
		});
		expect(forkOption.hasAttribute("data-combobox-disabled")).toBe(true);
	});
});
