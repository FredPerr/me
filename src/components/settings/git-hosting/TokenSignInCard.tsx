import { Alert, Button, Group, PasswordInput, Stack, Text, TextInput } from "@mantine/core";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { HostKindView } from "@/bindings";
import { useSignInWithToken } from "@/hooks/gitHosting/useGitHostingMutations";
import { describeError } from "@/infra/commandError";
import { GitHostCardLayout, GitHostIcon } from "./GitHostCardLayout";

const BUTTON_ICON_SIZE = 16;
const LINK_ICON_SIZE = 14;
const API_TOKENS_URL = "https://id.atlassian.com/manage-profile/security/api-tokens";

type TokenSignInFormProps = {
	host: HostKindView;
	onClose: () => void;
};

/** Username and API token, checked by the Git host before anything is saved. */
export function TokenSignInForm({ host, onClose }: TokenSignInFormProps) {
	const { t } = useTranslation();
	const signIn = useSignInWithToken();
	const [username, setUsername] = useState("");
	const [token, setToken] = useState("");
	const isComplete = username.trim() !== "" && token.trim() !== "";

	function handleSignIn() {
		signIn.mutate({ host, username, token }, { onSuccess: onClose });
	}

	return (
		<Stack gap="xs" mt="md">
			<Text size="sm">{t("settings.gitProvider.tokenInstructions")}</Text>
			<Button
				variant="subtle"
				size="xs"
				leftSection={<ArrowSquareOutIcon size={LINK_ICON_SIZE} />}
				onClick={() => openUrl(API_TOKENS_URL)}
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
			{signIn.isError && (
				<Alert color="red" title={t("common.error")}>
					{describeError(signIn.error)}
				</Alert>
			)}
			<Group justify="flex-end">
				<Button variant="subtle" onClick={onClose} disabled={signIn.isPending}>
					{t("common.cancel")}
				</Button>
				<Button onClick={handleSignIn} loading={signIn.isPending} disabled={!isComplete}>
					{t("settings.gitProvider.connect")}
				</Button>
			</Group>
		</Stack>
	);
}

/** A Git host signed in with a username and an API token (Bitbucket). */
export function TokenSignInCard({ host }: { host: HostKindView }) {
	const { t } = useTranslation();
	const [showsForm, setShowsForm] = useState(false);

	return (
		<GitHostCardLayout
			host={host}
			signedIn={null}
			action={
				<Button
					leftSection={<GitHostIcon host={host} size={BUTTON_ICON_SIZE} />}
					onClick={() => setShowsForm((shown) => !shown)}
				>
					{t("settings.gitProvider.connect")}
				</Button>
			}
		>
			{showsForm && <TokenSignInForm host={host} onClose={() => setShowsForm(false)} />}
		</GitHostCardLayout>
	);
}
