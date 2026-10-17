// @vitest-environment jsdom
import { MantineProvider } from "@mantine/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { PlacementView, ProjectView, WorkbenchView } from "@/bindings";
import "@/i18n";
import { ProjectEditProvider } from "@/components/settings/ProjectEditProvider";
import { buildProject } from "@/test-utils/workspaceBuilders";
import { ProjectTabs } from "./ProjectTabs";

const workbenchCommands = vi.hoisted(() => ({
	get: vi.fn(),
	setLastProject: vi.fn(),
	setPlacement: vi.fn(),
	reorder: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	commands: {
		workbenchGet: workbenchCommands.get,
		workbenchSetLastProject: workbenchCommands.setLastProject,
		workbenchSetPlacement: workbenchCommands.setPlacement,
		workbenchReorder: workbenchCommands.reorder,
	},
	events: {},
}));

// The per-project body pulls in the whole board; the bar's behaviour is the panel's presence.
vi.mock("./ProjectTabPanel", () => ({
	ProjectTabPanel: ({ projectTag }: { projectTag: string }) => (
		<div data-testid={`panel-${projectTag}`} />
	),
}));

const PINNED_TAG = "api";
const SHOWN_TAG = "website";
const HIDDEN_TAG = "legacy";
const PINNED_NAME = "API";
const SHOWN_NAME = "Website";
const HIDDEN_NAME = "Legacy";

const PINNED_PROJECT = buildProject({ tag: PINNED_TAG, name: PINNED_NAME });
const SHOWN_PROJECT = buildProject({ tag: SHOWN_TAG, name: SHOWN_NAME });
const HIDDEN_PROJECT = buildProject({ tag: HIDDEN_TAG, name: HIDDEN_NAME });
const ALL_PROJECTS = [PINNED_PROJECT, SHOWN_PROJECT, HIDDEN_PROJECT];

function entry(projectTag: string, placement: PlacementView) {
	return { projectTag, placement };
}

function buildWorkbench(lastProjectTag: string | null): WorkbenchView {
	return {
		entries: [entry(PINNED_TAG, "pinned"), entry(SHOWN_TAG, "shown"), entry(HIDDEN_TAG, "hidden")],
		lastProjectTag,
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

describe("ProjectTabs", () => {
	function renderTabs(projects: ProjectView[] = ALL_PROJECTS) {
		const queryClient = new QueryClient({
			defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
		});
		const wrapper = ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={queryClient}>
				<MantineProvider>
					<ProjectEditProvider>{children}</ProjectEditProvider>
				</MantineProvider>
			</QueryClientProvider>
		);
		render(<ProjectTabs projects={projects} />, { wrapper });
	}

	beforeEach(() => {
		workbenchCommands.get.mockReset().mockResolvedValue(buildWorkbench(SHOWN_TAG));
		workbenchCommands.setLastProject.mockReset().mockResolvedValue(buildWorkbench(SHOWN_TAG));
		workbenchCommands.setPlacement.mockReset().mockResolvedValue(buildWorkbench(SHOWN_TAG));
	});

	afterEach(cleanup);

	it("renders pinned and shown projects as tabs but not hidden ones", async () => {
		renderTabs();

		expect(await screen.findByRole("tab", { name: PINNED_NAME })).toBeDefined();
		expect(screen.getByRole("tab", { name: SHOWN_NAME })).toBeDefined();
		expect(screen.queryByRole("tab", { name: HIDDEN_NAME })).toBeNull();
	});

	it("activates the last-open project when it is visible", async () => {
		renderTabs();

		const shownTab = await screen.findByRole("tab", { name: SHOWN_NAME });

		expect(shownTab.getAttribute("aria-selected")).toBe("true");
	});

	it("falls back to the first pinned tab when last-open is null", async () => {
		workbenchCommands.get.mockResolvedValue(buildWorkbench(null));
		renderTabs();

		const pinnedTab = await screen.findByRole("tab", { name: PINNED_NAME });

		expect(pinnedTab.getAttribute("aria-selected")).toBe("true");
	});

	it("records the last-open project when switching tabs", async () => {
		renderTabs();

		await userEvent.click(await screen.findByRole("tab", { name: PINNED_NAME }));

		await waitFor(() => expect(workbenchCommands.setLastProject).toHaveBeenCalledWith(PINNED_TAG));
	});

	it("shows the hidden count on the +N button", async () => {
		renderTabs();

		const hiddenButton = await screen.findByRole("button", { name: /hidden projects/i });

		expect(hiddenButton.textContent).toBe("+1");
	});

	it("omits the +N button when nothing is hidden", async () => {
		workbenchCommands.get.mockResolvedValue({
			entries: [entry(PINNED_TAG, "pinned"), entry(SHOWN_TAG, "shown")],
			lastProjectTag: SHOWN_TAG,
		});
		renderTabs([PINNED_PROJECT, SHOWN_PROJECT]);

		await screen.findByRole("tab", { name: SHOWN_NAME });

		expect(screen.queryByRole("button", { name: /hidden projects/i })).toBeNull();
	});

	it("shows a hidden project when its Show action is chosen", async () => {
		renderTabs();

		await userEvent.click(await screen.findByRole("button", { name: /hidden projects/i }));
		await userEvent.click(await screen.findByRole("menuitem", { name: /show legacy/i }));

		await waitFor(() =>
			expect(workbenchCommands.setPlacement).toHaveBeenCalledWith(HIDDEN_TAG, "shown"),
		);
	});
});
