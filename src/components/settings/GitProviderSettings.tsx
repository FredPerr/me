import {
	Alert,
	Avatar,
	Button,
	Code,
	CopyButton,
	Group,
	Paper,
	Stack,
	Text,
	Title,
	Tooltip,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { ArrowSquareOutIcon, CheckIcon, CopyIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import { useGitProviderAuth } from "@/hooks/useGitProviderAuth";

const GITHUB_PROVIDER = "github" as const;

export function GitProviderSettings() {
	const { t } = useTranslation();
	const { status, account, deviceAuthorization, error, loading, connect, disconnect, cancel } =
		useGitProviderAuth(GITHUB_PROVIDER);

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

	async function handleDisconnect() {
		try {
			await disconnect();
			notifications.show({
				title: t("settings.gitProvider.disconnectedTitle"),
				message: t("settings.gitProvider.disconnectedMessage"),
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

	return (
		<Stack gap="sm">
			<Title order={3}>{t("settings.gitProvider.title")}</Title>
			<Text size="sm" c="dimmed">
				{t("settings.gitProvider.description")}
			</Text>

			<Paper withBorder p="md" radius="md">
				<Group justify="space-between" wrap="nowrap">
					<Group gap="sm" wrap="nowrap">
						{status === "connected" && account?.avatarUrl ? (
							<Avatar src={account.avatarUrl} radius="xl" size="md" alt={account.login} />
						) : (
							<GithubLogoIcon size={28} weight="fill" />
						)}
						<Stack gap={0}>
							<Text fw={500}>{t("settings.gitProvider.github")}</Text>
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
					) : status === "polling" || status === "awaitingCode" ? (
						<Button variant="subtle" onClick={cancel}>
							{t("common.cancel")}
						</Button>
					) : (
						<Button
							leftSection={<GithubLogoIcon size={16} weight="fill" />}
							onClick={handleConnect}
						>
							{t("settings.gitProvider.connect")}
						</Button>
					)}
				</Group>

				{deviceAuthorization && (status === "polling" || status === "awaitingCode") && (
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

				{status === "error" && error && (
					<Alert color="red" mt="md" title={t("common.error")}>
						{error === "expired"
							? t("settings.gitProvider.expired")
							: error === "denied"
								? t("settings.gitProvider.denied")
								: t("settings.gitProvider.connectError")}
					</Alert>
				)}
			</Paper>
		</Stack>
	);
}
