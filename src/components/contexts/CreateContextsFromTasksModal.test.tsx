// @vitest-environment jsdom
import { MantineProvider } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type {
	AppSettingsView,
	AssignMeResultView,
	ConnectionView,
	PendingTaskView,
	ProjectView,
	Task,
} from "@/bindings";
import "@/i18n";
import { buildProject } from "@/test-utils/workspaceBuilders";
import { CreateContextsFromTasksModal } from "./CreateContextsFromTasksModal";

const CONNECTION_ID = "teamwork:acme.teamwork.com";
const FIRST_TASK_ID = "101";
const FIRST_TASK_TITLE = "Add the login page";
const CREATED_BRANCH = "feature/login";
const REMOTE_PROJECT_ID = "42";
const ASSIGNED: AssignMeResultView = { failed: [] };

const trackerCommands = vi.hoisted(() => ({
	pendingTasks: vi.fn(),
	listConnections: vi.fn(),
	suggestBranchName: vi.fn(),
	createContext: vi.fn(),
	assignMeToTasks: vi.fn(),
	settingsGet: vi.fn(),
	settingsSetAssignMeToTasks: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	commands: {
		workTrackingPendingTasks: trackerCommands.pendingTasks,
		workTrackingListConnections: trackerCommands.listConnections,
		workspaceSuggestBranchName: trackerCommands.suggestBranchName,
		workspaceCreateContext: trackerCommands.createContext,
		workTrackingAssignMeToTasks: trackerCommands.assignMeToTasks,
		settingsGet: trackerCommands.settingsGet,
		settingsSetAssignMeToTasks: trackerCommands.settingsSetAssignMeToTasks,
	},
}));

function buildTask(overrides?: Partial<Task>): Task {
	return {
		id: FIRST_TASK_ID,
		projectId: REMOTE_PROJECT_ID,
		groupId: null,
		parentId: null,
		title: FIRST_TASK_TITLE,
		description: null,
		status: "todo",
		rawStatus: "new",
		priority: "none",
		rawPriority: null,
		assignees: [],
		labels: [],
		dueDate: null,
		url: "https://acme.teamwork.com/app/tasks/101",
		updatedAt: null,
		...overrides,
	};
}

function buildPendingTask(task: Task): PendingTaskView {
	return { connectionId: CONNECTION_ID, remoteProjectId: REMOTE_PROJECT_ID, task };
}

function buildConnection(): ConnectionView {
	return {
		id: CONNECTION_ID,
		kind: "teamwork",
		baseUrl: "https://acme.teamwork.com",
		displayName: "Acme",
		credentialConfigured: true,
	};
}

function buildSettings(assignMeToTasks: boolean): AppSettingsView {
	return { assignMeToTasks };
}

const project: ProjectView = buildProject({
	remoteProjectLinks: [
		{
			providerKind: "teamwork",
			connectionId: CONNECTION_ID,
			remoteProjectId: REMOTE_PROJECT_ID,
			remoteProjectName: "Website",
			remoteProjectUrl: null,
			linkedAt: "2026-01-01T00:00:00Z",
		},
	],
	contexts: [
		{
			id: "context-default",
			name: "default",
			folder: "default",
			folderPath: "/projects/website",
			idePath: "/projects/website",
			status: "idle",
			isDefault: true,
			isStatic: true,
			baseContextId: null,
			branches: [],
			origin: { kind: "manual" },
			agentWork: null,
		},
	],
});

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
	// The configure step renders an autosize Textarea that listens for font loads.
	if (!document.fonts) {
		Object.defineProperty(document, "fonts", {
			configurable: true,
			value: { addEventListener: () => {}, removeEventListener: () => {} },
		});
	}
});

describe("CreateContextsFromTasksModal", () => {
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
		render(<CreateContextsFromTasksModal project={project} onClose={onClose} />, { wrapper });
		return { onClose };
	}

	/** Walks from the select step to the create step with the first task picked. */
	async function selectFirstTaskAndContinue() {
		const taskCheckbox = await screen.findByRole("checkbox", {
			name: `Select task ${FIRST_TASK_TITLE}`,
		});
		await userEvent.click(taskCheckbox);
		await userEvent.click(screen.getByRole("button", { name: /Next/ }));
		await screen.findByRole("checkbox", { name: /Assign me to these tasks/ });
	}

	function createButton() {
		return screen.getByRole("button", { name: /Create 1 context/ });
	}

	beforeEach(() => {
		trackerCommands.pendingTasks.mockReset().mockResolvedValue({
			tasks: [buildPendingTask(buildTask())],
			unavailableConnections: [],
		});
		trackerCommands.listConnections.mockReset().mockResolvedValue([buildConnection()]);
		trackerCommands.suggestBranchName.mockReset().mockResolvedValue(CREATED_BRANCH);
		trackerCommands.createContext.mockReset().mockResolvedValue(project);
		trackerCommands.assignMeToTasks.mockReset().mockResolvedValue(ASSIGNED);
		trackerCommands.settingsGet.mockReset().mockResolvedValue(buildSettings(true));
		trackerCommands.settingsSetAssignMeToTasks
			.mockReset()
			.mockImplementation((value: boolean) => Promise.resolve(buildSettings(value)));
	});

	afterEach(cleanup);

	it("assigns the current user to the created tasks when the remembered setting is on", async () => {
		renderModal();
		await selectFirstTaskAndContinue();

		await userEvent.click(createButton());

		await waitFor(() =>
			expect(trackerCommands.assignMeToTasks).toHaveBeenCalledWith([
				{ connectionId: CONNECTION_ID, taskId: FIRST_TASK_ID },
			]),
		);
	});

	it("does not assign anyone when the checkbox is unchecked", async () => {
		renderModal();
		await selectFirstTaskAndContinue();

		await userEvent.click(screen.getByRole("checkbox", { name: /Assign me to these tasks/ }));
		await userEvent.click(createButton());

		await waitFor(() => expect(trackerCommands.createContext).toHaveBeenCalled());
		expect(trackerCommands.assignMeToTasks).not.toHaveBeenCalled();
	});

	it("remembers the checkbox by writing it back when toggled", async () => {
		renderModal();
		await selectFirstTaskAndContinue();

		await userEvent.click(screen.getByRole("checkbox", { name: /Assign me to these tasks/ }));

		await waitFor(() =>
			expect(trackerCommands.settingsSetAssignMeToTasks).toHaveBeenCalledWith(false),
		);
	});

	it("defaults the checkbox to the remembered off setting", async () => {
		trackerCommands.settingsGet.mockResolvedValue(buildSettings(false));
		renderModal();
		await selectFirstTaskAndContinue();

		const checkbox = screen.getByRole("checkbox", { name: /Assign me to these tasks/ });
		if (!(checkbox instanceof HTMLInputElement)) throw new Error("expected a checkbox input");
		await waitFor(() => expect(checkbox.checked).toBe(false));
	});

	it("surfaces the tasks that could not be assigned while keeping the contexts", async () => {
		trackerCommands.assignMeToTasks.mockResolvedValue({
			failed: [{ connectionId: CONNECTION_ID, taskId: FIRST_TASK_ID }],
		} satisfies AssignMeResultView);
		const { onClose } = renderModal();
		await selectFirstTaskAndContinue();

		await userEvent.click(createButton());

		// The warning names the task so the user can assign themselves by hand.
		const warning = await screen.findByText(/could not be assigned/);
		expect(warning.textContent).toContain(FIRST_TASK_TITLE);
		// The contexts were still created and the modal closes.
		expect(trackerCommands.createContext).toHaveBeenCalled();
		await waitFor(() => expect(onClose).toHaveBeenCalled());
	});
});
