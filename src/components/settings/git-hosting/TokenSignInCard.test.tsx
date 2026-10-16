// @vitest-environment jsdom
import { MantineProvider } from "@mantine/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { GitHostingCommandError, HostedAccountView } from "@/bindings";
import "@/i18n";
import { TokenSignInForm } from "./TokenSignInCard";

const signInCommands = vi.hoisted(() => ({ signInWithToken: vi.fn() }));

vi.mock("@/bindings", () => ({
	commands: { gitHostingSignInWithToken: signInCommands.signInWithToken },
}));

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

const HOST = "bitbucket";
const USERNAME = "alice";
const TOKEN = "api-token";
const SIGNED_IN_ACCOUNT: HostedAccountView = {
	host: HOST,
	login: USERNAME,
	name: null,
	avatarUrl: null,
};
const UNAUTHORIZED_ERROR: GitHostingCommandError = {
	kind: "unauthorized",
	message: "The Git host rejected the credentials.",
	retryAfterSeconds: null,
	status: null,
};

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
});

describe("TokenSignInForm", () => {
	const onClose = vi.fn();

	function renderForm() {
		const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
		const wrapper = ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={queryClient}>
				<MantineProvider>{children}</MantineProvider>
			</QueryClientProvider>
		);
		render(<TokenSignInForm host={HOST} onClose={onClose} />, { wrapper });
	}

	async function signIn() {
		await userEvent.type(screen.getByLabelText("Username"), USERNAME);
		await userEvent.type(screen.getByLabelText("API token"), TOKEN);
		await userEvent.click(screen.getByRole("button", { name: "Connect" }));
	}

	beforeEach(() => {
		onClose.mockClear();
		signInCommands.signInWithToken.mockReset().mockResolvedValue(SIGNED_IN_ACCOUNT);
	});

	afterEach(cleanup);

	it("signs in with the entered username and token, then closes", async () => {
		renderForm();

		await signIn();

		expect(signInCommands.signInWithToken).toHaveBeenCalledWith(HOST, USERNAME, TOKEN);
		expect(onClose).toHaveBeenCalledOnce();
	});

	it("shows the Git host's message when the credentials are refused", async () => {
		signInCommands.signInWithToken.mockRejectedValue(UNAUTHORIZED_ERROR);
		renderForm();

		await signIn();

		expect(await screen.findByText(UNAUTHORIZED_ERROR.message)).toBeDefined();
		expect(onClose).not.toHaveBeenCalled();
	});
});
