import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { type BrowserSignInStepView, commands, type HostKindView } from "@/bindings";
import { describeError } from "@/infra/commandError";
import { GIT_HOSTING_QUERY_KEY } from "./gitHostingQueryKeys";

const MILLISECONDS_PER_SECOND = 1000;

export type BrowserSignInFailure =
	| { reason: "expired" }
	| { reason: "denied" }
	| { reason: "error"; message: string };

export type BrowserSignInState =
	| { status: "idle" }
	| { status: "starting" }
	| { status: "waiting"; step: BrowserSignInStepView }
	| { status: "failed"; failure: BrowserSignInFailure };

const IDLE: BrowserSignInState = { status: "idle" };

/**
 * Drives a browser sign-in: Rust starts it and answers each check, while the
 * wait between checks and the cancel button stay here.
 */
export function useBrowserSignIn(host: HostKindView) {
	const queryClient = useQueryClient();
	const [state, setState] = useState<BrowserSignInState>(IDLE);
	const pollTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
	// Bumped by every start and cancel, so answers to an older attempt are dropped.
	const attempt = useRef(0);

	const clearPollTimer = useCallback(() => {
		if (pollTimer.current) clearTimeout(pollTimer.current);
		pollTimer.current = null;
	}, []);

	useEffect(
		() => () => {
			attempt.current += 1;
			clearPollTimer();
		},
		[clearPollTimer],
	);

	const fail = useCallback((failure: BrowserSignInFailure) => {
		setState({ status: "failed", failure });
	}, []);

	const schedulePoll = useCallback(
		(currentAttempt: number, deviceCode: string, intervalSeconds: number) => {
			const poll = async () => {
				try {
					const progress = await commands.gitHostingPollSignIn(host, deviceCode);
					if (currentAttempt !== attempt.current) return;
					switch (progress.status) {
						case "authorized":
							await queryClient.invalidateQueries({ queryKey: GIT_HOSTING_QUERY_KEY });
							setState(IDLE);
							return;
						case "pending":
							schedulePoll(currentAttempt, deviceCode, intervalSeconds);
							return;
						case "slowDown":
							schedulePoll(currentAttempt, deviceCode, progress.intervalSeconds);
							return;
						case "expired":
							fail({ reason: "expired" });
							return;
						case "denied":
							fail({ reason: "denied" });
							return;
					}
				} catch (pollError) {
					if (currentAttempt !== attempt.current) return;
					fail({ reason: "error", message: describeError(pollError) });
				}
			};
			pollTimer.current = setTimeout(poll, intervalSeconds * MILLISECONDS_PER_SECOND);
		},
		[host, queryClient, fail],
	);

	const start = useCallback(async () => {
		attempt.current += 1;
		const currentAttempt = attempt.current;
		clearPollTimer();
		setState({ status: "starting" });
		try {
			const step = await commands.gitHostingStartSignIn(host);
			if (currentAttempt !== attempt.current) return;
			setState({ status: "waiting", step });
			schedulePoll(currentAttempt, step.deviceCode, step.intervalSeconds);
		} catch (startError) {
			if (currentAttempt !== attempt.current) return;
			fail({ reason: "error", message: describeError(startError) });
		}
	}, [host, clearPollTimer, schedulePoll, fail]);

	const cancel = useCallback(() => {
		attempt.current += 1;
		clearPollTimer();
		setState(IDLE);
	}, [clearPollTimer]);

	return { state, start, cancel };
}
