import { useCallback, useEffect, useRef, useState } from "react";
import { type GitAccount, GitAccountRepository } from "@/models/git-provider/GitAccount";
import type { DeviceAuthorization, GitProviderId } from "@/models/git-provider/GitProvider";
import { getGitProvider } from "@/models/git-provider/GitProviderRegistry";

export type AuthStatus = "idle" | "awaitingCode" | "polling" | "connected" | "error";

const SECONDS_TO_MS = 1000;

type UseGitProviderAuthResult = {
	status: AuthStatus;
	account: GitAccount | null;
	deviceAuthorization: DeviceAuthorization | null;
	error: string | null;
	loading: boolean;
	connect: () => Promise<void>;
	disconnect: () => Promise<void>;
	cancel: () => void;
};

export function useGitProviderAuth(providerId: GitProviderId): UseGitProviderAuthResult {
	const provider = getGitProvider(providerId);

	const [status, setStatus] = useState<AuthStatus>("idle");
	const [account, setAccount] = useState<GitAccount | null>(null);
	const [deviceAuthorization, setDeviceAuthorization] = useState<DeviceAuthorization | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [loading, setLoading] = useState(true);

	const pollTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
	const cancelled = useRef(false);

	const clearPollTimer = useCallback(() => {
		if (pollTimer.current) {
			clearTimeout(pollTimer.current);
			pollTimer.current = null;
		}
	}, []);

	const refreshConnection = useCallback(async () => {
		const connected = await provider.isConnected();
		if (!connected) {
			setAccount(null);
			setStatus("idle");
			return;
		}
		const stored = await GitAccountRepository.find(providerId);
		setAccount(stored);
		setStatus("connected");
	}, [provider, providerId]);

	useEffect(() => {
		refreshConnection().finally(() => setLoading(false));
		return () => {
			cancelled.current = true;
			clearPollTimer();
		};
	}, [refreshConnection, clearPollTimer]);

	const finishConnection = useCallback(async () => {
		const user = await provider.getAuthenticatedUser();
		const connectedAccount: GitAccount = {
			provider: providerId,
			login: user.login,
			name: user.name,
			avatarUrl: user.avatarUrl,
		};
		await GitAccountRepository.save(connectedAccount);
		setAccount(connectedAccount);
		setDeviceAuthorization(null);
		setStatus("connected");
	}, [provider, providerId]);

	const poll = useCallback(
		async (deviceCode: string, intervalSeconds: number) => {
			if (cancelled.current) return;
			try {
				const result = await provider.pollForAccessToken(deviceCode);
				if (cancelled.current) return;

				switch (result.status) {
					case "authorized":
						await finishConnection();
						return;
					case "pending":
						pollTimer.current = setTimeout(
							() => poll(deviceCode, intervalSeconds),
							intervalSeconds * SECONDS_TO_MS,
						);
						return;
					case "slowDown":
						pollTimer.current = setTimeout(
							() => poll(deviceCode, result.interval),
							result.interval * SECONDS_TO_MS,
						);
						return;
					case "expired":
						setError("expired");
						setStatus("error");
						setDeviceAuthorization(null);
						return;
					case "denied":
						setError("denied");
						setStatus("error");
						setDeviceAuthorization(null);
						return;
				}
			} catch (pollError) {
				if (cancelled.current) return;
				setError(String(pollError));
				setStatus("error");
				setDeviceAuthorization(null);
			}
		},
		[provider, finishConnection],
	);

	const connect = useCallback(async () => {
		cancelled.current = false;
		clearPollTimer();
		setError(null);
		setStatus("awaitingCode");
		try {
			const authorization = await provider.startDeviceAuthorization();
			setDeviceAuthorization(authorization);
			setStatus("polling");
			pollTimer.current = setTimeout(
				() => poll(authorization.deviceCode, authorization.interval),
				authorization.interval * SECONDS_TO_MS,
			);
		} catch (startError) {
			setError(String(startError));
			setStatus("error");
			setDeviceAuthorization(null);
		}
	}, [provider, poll, clearPollTimer]);

	const disconnect = useCallback(async () => {
		clearPollTimer();
		await provider.disconnect();
		await GitAccountRepository.remove(providerId);
		setAccount(null);
		setDeviceAuthorization(null);
		setError(null);
		setStatus("idle");
	}, [provider, providerId, clearPollTimer]);

	const cancel = useCallback(() => {
		cancelled.current = true;
		clearPollTimer();
		setDeviceAuthorization(null);
		setError(null);
		setStatus("idle");
	}, [clearPollTimer]);

	return {
		status,
		account,
		deviceAuthorization,
		error,
		loading,
		connect,
		disconnect,
		cancel,
	};
}
