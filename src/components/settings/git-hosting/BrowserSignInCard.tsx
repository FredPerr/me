import { Alert, Button, Code, CopyButton, Group, Stack, Text, Tooltip } from "@mantine/core";
import { ArrowSquareOutIcon, CheckIcon, CopyIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import type { BrowserSignInStepView, HostKindView } from "@/bindings";
import { type BrowserSignInFailure, useBrowserSignIn } from "@/hooks/gitHosting/useBrowserSignIn";
import { GitHostCardLayout, GitHostIcon } from "./GitHostCardLayout";

const BUTTON_ICON_SIZE = 16;
const COPY_ICON_SIZE = 14;

function BrowserStep({ step }: { step: BrowserSignInStepView }) {
	const { t } = useTranslation();
	return (
		<Stack gap="xs" mt="md">
			<Text size="sm">{t("settings.gitProvider.deviceInstructions")}</Text>
			<Group gap="xs">
				<Code fz="lg" fw={700}>
					{step.userCode}
				</Code>
				<CopyButton value={step.userCode}>
					{({ copied, copy }) => (
						<Tooltip label={copied ? t("common.copied") : t("settings.gitProvider.copyCode")}>
							<Button
								size="xs"
								variant="subtle"
								onClick={copy}
								leftSection={
									copied ? <CheckIcon size={COPY_ICON_SIZE} /> : <CopyIcon size={COPY_ICON_SIZE} />
								}
							>
								{copied ? t("common.copied") : t("settings.gitProvider.copyCode")}
							</Button>
						</Tooltip>
					)}
				</CopyButton>
			</Group>
			<Button
				variant="light"
				leftSection={<ArrowSquareOutIcon size={BUTTON_ICON_SIZE} />}
				onClick={() => openUrl(step.verificationUrl)}
			>
				{t("settings.gitProvider.openVerification")}
			</Button>
			<Text size="xs" c="dimmed">
				{t("settings.gitProvider.waitingForAuthorization")}
			</Text>
		</Stack>
	);
}

function FailureAlert({ failure }: { failure: BrowserSignInFailure }) {
	const { t } = useTranslation();
	const message =
		failure.reason === "expired"
			? t("settings.gitProvider.expired")
			: failure.reason === "denied"
				? t("settings.gitProvider.denied")
				: failure.message;
	return (
		<Alert color="red" mt="md" title={t("common.error")}>
			{message}
		</Alert>
	);
}

/** A Git host signed in by entering a code in the browser (GitHub). */
export function BrowserSignInCard({ host }: { host: HostKindView }) {
	const { t } = useTranslation();
	const { state, start, cancel } = useBrowserSignIn(host);
	const inProgress = state.status === "starting" || state.status === "waiting";

	return (
		<GitHostCardLayout
			host={host}
			signedIn={null}
			action={
				inProgress ? (
					<Button variant="subtle" onClick={cancel}>
						{t("common.cancel")}
					</Button>
				) : (
					<Button leftSection={<GitHostIcon host={host} size={BUTTON_ICON_SIZE} />} onClick={start}>
						{t("settings.gitProvider.connect")}
					</Button>
				)
			}
		>
			{state.status === "waiting" && <BrowserStep step={state.step} />}
			{state.status === "failed" && <FailureAlert failure={state.failure} />}
		</GitHostCardLayout>
	);
}
