import {
	Alert,
	Avatar,
	Button,
	Code,
	CopyButton,
	Group,
	Paper,
	PasswordInput,
	Stack,
	Text,
	TextInput,
	Tooltip,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import {
	ArrowSquareOutIcon,
	CheckIcon,
	CopyIcon,
	GitBranchIcon,
	GithubLogoIcon,
} from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useGitProviderAuth } from "@/hooks/useGitProviderAuth";
import type { GitProvider } from "@/models/git-provider/GitProvider";

type GitProviderCardProps = {
	provider: GitProvider;
};

/** Icon shown for a provider in the connected-accounts list. */
function ProviderIcon({ providerId, size }: { providerId: string; size: number }) {
	if (providerId === "github") {
		return <GithubLogoIcon size={size} weight="fill" />;
	}
	return <GitBranchIcon size={size} weight="fill" />;
}

/** A single connected-account card: GitHub (device flow) or Bitbucket (token). */
export function GitProviderCard({ provider }: GitProviderCardProps) {
	const { t } = useTranslation();
	const {
		status,
		authKind,
		account,
		deviceAuthorization,
		error,
		loading,
		connect,
		connectWithToken,
		disconnect,
		cancel,
	} = useGitProviderAuth(provider.id);

	const [username, setUsername] = useState("");
	const [token, setToken] = useState("");
	const [showTokenForm, setShowTokenForm] = useState(false);

	async function handleConnect() {
		try {
			await connect();
		} catch {
			notifications.show({
				title: t("common.error"),
				message: t("settings.gitProvider.connectError"),
				color: "red",
			});
		}
	}

	async function handleConnectWithToken() {
		try {
			await connectWithToken({ username: username.trim(), token: token.trim() });
			setShowTokenForm(false);
			setUsername("");
			setToken("");
		} catch {
			notifications.show({
				title: t("common.error"),
				message: t("settings.gitProvider.tokenConnectError"),
				color: "red",
			});
		}
	}

	async function handleDisconnect() {
		try {
			await disconnect();
			setShowTokenForm(false);
			notifications.show({
				title: t("settings.gitProvider.disconnectedTitle"),
				message: t("settings.gitProvider.disconnectedMessage", { provider: provider.displayName }),
				color: "green",
			});
		} catch {
			notifications.show({
				title: t("common.error"),
				message: t("settings.gitProvider.disconnectError"),
				color: "red",
			});
		}
	}

	if (loading) {
		return null;
	}

	const busy = status === "polling" || status === "awaitingCode";

	return (
		<Paper withBorder p="md" radius="md">
			<Group justify="space-between" wrap="nowrap">
				<Group gap="sm" wrap="nowrap">
					{status === "connected" && account?.avatarUrl ? (
						<Avatar src={account.avatarUrl} radius="xl" size="md" alt={account.login} />
					) : (
						<ProviderIcon providerId={provider.id} size={28} />
					)}
					<Stack gap={0}>
						<Text fw={500}>{provider.displayName}</Text>
						{status === "connected" && account ? (
							<Text size="sm" c="dimmed">
								{t("settings.gitProvider.connectedAs", { login: account.login })}
							</Text>
						) : (
							<Text size="sm" c="dimmed">
								{t("settings.gitProvider.notConnected")}
							</Text>
						)}
					</Stack>
				</Group>

				{status === "connected" ? (
					<Button variant="light" color="red" onClick={handleDisconnect}>
						{t("settings.gitProvider.disconnect")}
					</Button>
				) : busy && authKind === "deviceFlow" ? (
					<Button variant="subtle" onClick={cancel}>
						{t("common.cancel")}
					</Button>
				) : authKind === "deviceFlow" ? (
					<Button
						leftSection={<ProviderIcon providerId={provider.id} size={16} />}
						onClick={handleConnect}
					>
						{t("settings.gitProvider.connect")}
					</Button>
				) : (
					<Button
						leftSection={<ProviderIcon providerId={provider.id} size={16} />}
						onClick={() => setShowTokenForm((open) => !open)}
					>
						{t("settings.gitProvider.connect")}
					</Button>
				)}
			</Group>

			{authKind === "deviceFlow" &&
				deviceAuthorization &&
				(status === "polling" || status === "awaitingCode") && (
					<Stack gap="xs" mt="md">
						<Text size="sm">{t("settings.gitProvider.deviceInstructions")}</Text>
						<Group gap="xs">
							<Code fz="lg" fw={700}>
								{deviceAuthorization.userCode}
							</Code>
							<CopyButton value={deviceAuthorization.userCode}>
								{({ copied, copy }) => (
									<Tooltip label={copied ? t("common.copied") : t("settings.gitProvider.copyCode")}>
										<Button
											size="xs"
											variant="subtle"
											onClick={copy}
											leftSection={copied ? <CheckIcon size={14} /> : <CopyIcon size={14} />}
										>
											{copied ? t("common.copied") : t("settings.gitProvider.copyCode")}
										</Button>
									</Tooltip>
								)}
							</CopyButton>
						</Group>
						<Button
							variant="light"
							leftSection={<ArrowSquareOutIcon size={16} />}
							onClick={() => openUrl(deviceAuthorization.verificationUri)}
						>
							{t("settings.gitProvider.openVerification")}
						</Button>
						<Text size="xs" c="dimmed">
							{t("settings.gitProvider.waitingForAuthorization")}
						</Text>
					</Stack>
				)}

			{authKind === "token" && status !== "connected" && showTokenForm && (
				<Stack gap="xs" mt="md">
					<Text size="sm">{t("settings.gitProvider.tokenInstructions")}</Text>
					<Button
						variant="subtle"
						size="xs"
						leftSection={<ArrowSquareOutIcon size={14} />}
						onClick={() => openUrl("https://id.atlassian.com/manage-profile/security/api-tokens")}
						style={{ alignSelf: "flex-start" }}
					>
						{t("settings.gitProvider.createAppPassword")}
					</Button>
					<TextInput
						label={t("settings.gitProvider.usernameLabel")}
						placeholder={t("settings.gitProvider.usernamePlaceholder")}
						value={username}
						onChange={(event) => setUsername(event.currentTarget.value)}
					/>
					<PasswordInput
						label={t("settings.gitProvider.tokenLabel")}
						placeholder={t("settings.gitProvider.tokenPlaceholder")}
						value={token}
						onChange={(event) => setToken(event.currentTarget.value)}
					/>
					<Group justify="flex-end">
						<Button variant="subtle" onClick={() => setShowTokenForm(false)} disabled={busy}>
							{t("common.cancel")}
						</Button>
						<Button
							onClick={handleConnectWithToken}
							loading={busy}
							disabled={!username.trim() || !token.trim()}
						>
							{t("settings.gitProvider.connect")}
						</Button>
					</Group>
				</Stack>
			)}

			{status === "error" && error && (
				<Alert color="red" mt="md" title={t("common.error")}>
					{error === "expired"
						? t("settings.gitProvider.expired")
						: error === "denied"
							? t("settings.gitProvider.denied")
							: authKind === "token"
								? t("settings.gitProvider.tokenConnectError")
								: t("settings.gitProvider.connectError")}
				</Alert>
			)}
		</Paper>
	);
}
