// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BrowserSignInStepView, GitHostingCommandError, SignInProgressView } from "@/bindings";
import { HOSTED_ACCOUNTS_QUERY_KEY } from "./gitHostingQueryKeys";
import { useBrowserSignIn } from "./useBrowserSignIn";

const signInCommands = vi.hoisted(() => ({
	start: vi.fn(),
	poll: vi.fn(),
}));

vi.mock("@/bindings", () => ({
	commands: {
		gitHostingStartSignIn: signInCommands.start,
		gitHostingPollSignIn: signInCommands.poll,
	},
}));

const HOST = "github";
const INTERVAL_SECONDS = 5;
const SLOWER_INTERVAL_SECONDS = 10;
const MILLISECONDS_PER_SECOND = 1000;
const DEVICE_CODE = "device-code-1";
const PENDING: SignInProgressView = { status: "pending" };
const TIMEOUT_ERROR: GitHostingCommandError = {
	kind: "timeout",
	message: "The request to the Git host timed out.",
	retryAfterSeconds: null,
	status: null,
};

function buildStep(overrides?: Partial<BrowserSignInStepView>): BrowserSignInStepView {
	return {
		userCode: "WDJB-MJHT",
		verificationUrl: "https://github.com/login/device",
		deviceCode: DEVICE_CODE,
		intervalSeconds: INTERVAL_SECONDS,
		expiresInSeconds: 900,
		...overrides,
	};
}

async function waitSeconds(seconds: number) {
	await act(() => vi.advanceTimersByTimeAsync(seconds * MILLISECONDS_PER_SECOND));
}

describe("useBrowserSignIn", () => {
	let queryClient: QueryClient;

	async function renderStartedSignIn() {
		const wrapper = ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
		);
		const rendered = renderHook(() => useBrowserSignIn(HOST), { wrapper });
		await act(() => rendered.result.current.start());
		return rendered;
	}

	beforeEach(() => {
		vi.useFakeTimers();
		signInCommands.start.mockReset().mockResolvedValue(buildStep());
		signInCommands.poll.mockReset().mockResolvedValue(PENDING);
		queryClient = new QueryClient();
		queryClient.setQueryData(HOSTED_ACCOUNTS_QUERY_KEY, []);
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it("shows the browser step once started", async () => {
		const { result } = await renderStartedSignIn();

		expect(result.current.state).toEqual({ status: "waiting", step: buildStep() });
	});

	it("checks again after each interval while the sign-in is pending", async () => {
		await renderStartedSignIn();

		await waitSeconds(INTERVAL_SECONDS);
		await waitSeconds(INTERVAL_SECONDS);

		expect(signInCommands.poll).toHaveBeenCalledTimes(2);
		expect(signInCommands.poll).toHaveBeenCalledWith(HOST, DEVICE_CODE);
	});

	it("waits the longer interval the Git host asks for", async () => {
		signInCommands.poll.mockResolvedValueOnce({
			status: "slowDown",
			intervalSeconds: SLOWER_INTERVAL_SECONDS,
		});
		await renderStartedSignIn();
		await waitSeconds(INTERVAL_SECONDS);

		await waitSeconds(INTERVAL_SECONDS);
		const checksBeforeTheLongerInterval = signInCommands.poll.mock.calls.length;
		await waitSeconds(SLOWER_INTERVAL_SECONDS - INTERVAL_SECONDS);

		expect(checksBeforeTheLongerInterval).toBe(1);
		expect(signInCommands.poll).toHaveBeenCalledTimes(2);
	});

	it("stops checking once cancelled", async () => {
		const { result } = await renderStartedSignIn();

		act(() => result.current.cancel());
		await waitSeconds(INTERVAL_SECONDS);

		expect(signInCommands.poll).not.toHaveBeenCalled();
		expect(result.current.state).toEqual({ status: "idle" });
	});

	it("reports an expired code", async () => {
		signInCommands.poll.mockResolvedValueOnce({ status: "expired" });
		const { result } = await renderStartedSignIn();

		await waitSeconds(INTERVAL_SECONDS);

		expect(result.current.state).toEqual({ status: "failed", failure: { reason: "expired" } });
	});

	it("shows the Git host's message when a check fails", async () => {
		signInCommands.poll.mockRejectedValueOnce(TIMEOUT_ERROR);
		const { result } = await renderStartedSignIn();

		await waitSeconds(INTERVAL_SECONDS);

		expect(result.current.state).toEqual({
			status: "failed",
			failure: { reason: "error", message: TIMEOUT_ERROR.message },
		});
	});

	it("refetches the accounts once authorized", async () => {
		signInCommands.poll.mockResolvedValueOnce({ status: "authorized" });
		const { result } = await renderStartedSignIn();

		await waitSeconds(INTERVAL_SECONDS);

		expect(queryClient.getQueryState(HOSTED_ACCOUNTS_QUERY_KEY)?.isInvalidated).toBe(true);
		expect(result.current.state).toEqual({ status: "idle" });
	});
});
