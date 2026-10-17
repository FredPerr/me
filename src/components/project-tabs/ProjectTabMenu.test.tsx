// @vitest-environment jsdom
import { MantineProvider, Tabs, TabsList } from "@mantine/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectView, WorkbenchView } from "@/bindings";
import "@/i18n";
import { ProjectEditProvider } from "@/components/settings/ProjectEditProvider";
import { buildProject } from "@/test-utils/workspaceBuilders";
import { ProjectTab } from "./ProjectTab";
import { ProjectTabMenu } from "./ProjectTabMenu";

const PINNED_TAG = "api";
const SHOWN_TAG = "website";
const SHOWN_NAME = "Website";
const PINNED_NAME = "API";

const workbenchCommands = vi.hoisted(() => ({
	setPlacement: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	commands: {
		workbenchSetPlacement: workbenchCommands.setPlacement,
		workspaceUpdateProjectSettings: vi.fn(),
		workspaceAddProject: vi.fn(),
	},
	events: {},
}));

// The form's internals (git checks, folder picker) are out of this unit's scope;
// the subject is the menu opening the shared editor modal.
vi.mock("@/components/settings/ProjectForm", () => ({
	ProjectForm: ({ initialProject }: { initialProject?: ProjectView }) => (
		<div data-testid="project-form">{initialProject?.name}</div>
	),
}));

const EMPTY_WORKBENCH: WorkbenchView = { entries: [], lastProjectTag: null };

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

describe("ProjectTabMenu", () => {
	function renderMenu(project: ProjectView, pinned: boolean) {
		const queryClient = new QueryClient({
			defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
		});
		const wrapper = ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={queryClient}>
				<MantineProvider>
					<ProjectEditProvider>
						<Tabs value={project.tag}>
							<TabsList>{children}</TabsList>
						</Tabs>
					</ProjectEditProvider>
				</MantineProvider>
			</QueryClientProvider>
		);
		render(
			<ProjectTabMenu project={project} pinned={pinned}>
				<ProjectTab project={project} />
			</ProjectTabMenu>,
			{ wrapper },
		);
	}

	function openContextMenu(tabName: string) {
		const tab = screen.getByRole("tab", { name: tabName });
		fireEvent.contextMenu(tab);
	}

	beforeEach(() => {
		workbenchCommands.setPlacement.mockReset().mockResolvedValue(EMPTY_WORKBENCH);
	});

	afterEach(cleanup);

	const PIN_LABEL = "Pin";
	const UNPIN_LABEL = "Unpin";
	const HIDE_LABEL = "Hide";
	const PROJECT_SETTINGS_LABEL = "Project settings";

	it("offers Pin, Hide and Project settings for a shown tab", async () => {
		renderMenu(buildProject({ tag: SHOWN_TAG, name: SHOWN_NAME }), false);

		openContextMenu(SHOWN_NAME);

		const items = await screen.findAllByRole("menuitem");
		const labels = items.map((item) => item.textContent);
		expect(labels).toContain(PIN_LABEL);
		expect(labels).toContain(HIDE_LABEL);
		expect(labels).toContain(PROJECT_SETTINGS_LABEL);
	});

	it("offers Unpin for a pinned tab", async () => {
		renderMenu(buildProject({ tag: PINNED_TAG, name: PINNED_NAME }), true);

		openContextMenu(PINNED_NAME);

		expect(await screen.findByRole("menuitem", { name: UNPIN_LABEL })).toBeDefined();
		expect(screen.queryByRole("menuitem", { name: PIN_LABEL })).toBeNull();
	});

	it("pins a shown project when Pin is chosen", async () => {
		renderMenu(buildProject({ tag: SHOWN_TAG, name: SHOWN_NAME }), false);

		openContextMenu(SHOWN_NAME);
		await userEvent.click(await screen.findByRole("menuitem", { name: PIN_LABEL }));

		await waitFor(() =>
			expect(workbenchCommands.setPlacement).toHaveBeenCalledWith(SHOWN_TAG, "pinned"),
		);
	});

	it("opens the project editor when Project settings is chosen", async () => {
		renderMenu(buildProject({ tag: SHOWN_TAG, name: SHOWN_NAME }), false);

		openContextMenu(SHOWN_NAME);
		await userEvent.click(await screen.findByRole("menuitem", { name: PROJECT_SETTINGS_LABEL }));

		expect(await screen.findByTestId("project-form")).toBeDefined();
	});
});
