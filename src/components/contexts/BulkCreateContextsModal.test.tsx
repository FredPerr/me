// @vitest-environment jsdom
import { MantineProvider } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { AiAssistCommandError, ContextDraftView } from "@/bindings";
import "@/i18n";
import { buildProject, PROJECT_TAG } from "@/test-utils/workspaceBuilders";
import { BulkCreateContextsModal } from "./BulkCreateContextsModal";

const aiAssistCommands = vi.hoisted(() => ({
	suggestContexts: vi.fn(),
	cancelContexts: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	commands: {
		aiAssistSuggestContexts: aiAssistCommands.suggestContexts,
		aiAssistCancelContexts: aiAssistCommands.cancelContexts,
		// Used by useCreateContexts; never reached in these tests.
		workspaceCreateContext: vi.fn(),
		workspaceSuggestBranchName: vi.fn(),
	},
}));

const INSTRUCTION = "Add login and configure CI";
const ADD_LOGIN_DRAFT: ContextDraftView = {
	contextName: "Add login",
	branchName: "feature/add-login",
	prompt: "Implement the login page",
};
const CONFIGURE_CI_DRAFT: ContextDraftView = {
	contextName: "Configure CI",
	branchName: "feature/configure-ci",
	prompt: null,
};
const NO_LIST_ERROR: AiAssistCommandError = {
	kind: "noListFound",
	message: "Kiro did not return a usable list. Try rephrasing the instruction.",
};

const project = buildProject();

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
	// Mantine's autosize Textarea listens on document.fonts, absent in jsdom.
	if (!("fonts" in document)) {
		Object.defineProperty(document, "fonts", {
			value: { addEventListener: () => {}, removeEventListener: () => {} },
			configurable: true,
		});
	}
});

describe("BulkCreateContextsModal", () => {
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
		render(<BulkCreateContextsModal opened onClose={vi.fn()} project={project} />, { wrapper });
	}

	async function typeInstruction() {
		const textarea = screen.getByLabelText("Instruction", { exact: false });
		await userEvent.type(textarea, INSTRUCTION);
	}

	function generateButton() {
		return screen.getByRole("button", { name: "Generate" });
	}

	beforeEach(() => {
		aiAssistCommands.suggestContexts.mockReset();
		aiAssistCommands.cancelContexts.mockReset().mockResolvedValue(null);
	});

	afterEach(cleanup);

	it("generates drafts from the instruction and renders them", async () => {
		aiAssistCommands.suggestContexts.mockResolvedValue([ADD_LOGIN_DRAFT, CONFIGURE_CI_DRAFT]);
		renderModal();

		await typeInstruction();
		await userEvent.click(generateButton());

		expect(aiAssistCommands.suggestContexts).toHaveBeenCalledWith(PROJECT_TAG, INSTRUCTION);
		expect(await screen.findByDisplayValue(ADD_LOGIN_DRAFT.contextName)).toBeDefined();
		expect(screen.getByDisplayValue(ADD_LOGIN_DRAFT.branchName)).toBeDefined();
		expect(screen.getByDisplayValue(CONFIGURE_CI_DRAFT.contextName)).toBeDefined();
	});

	it("shows the message of a parse error", async () => {
		aiAssistCommands.suggestContexts.mockRejectedValue(NO_LIST_ERROR);
		renderModal();

		await typeInstruction();
		await userEvent.click(generateButton());

		expect(await screen.findByText(NO_LIST_ERROR.message)).toBeDefined();
	});

	it("cancels the suggestion in flight when the cancel button is clicked", async () => {
		// A suggestion that never resolves keeps the modal in the generating phase.
		aiAssistCommands.suggestContexts.mockReturnValue(new Promise(() => {}));
		renderModal();

		await typeInstruction();
		await userEvent.click(generateButton());

		const cancelButton = await screen.findByRole("button", { name: "Stop generating" });
		await userEvent.click(cancelButton);

		await waitFor(() => expect(aiAssistCommands.cancelContexts).toHaveBeenCalled());
	});
});
