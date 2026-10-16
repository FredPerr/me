import { Alert, Stack, Text, Title } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { HostKindView, SignedInHostView } from "@/bindings";
import { useHostedAccounts } from "@/hooks/gitHosting/useHostedAccounts";
import { describeError } from "@/infra/commandError";
import { BrowserSignInCard } from "./BrowserSignInCard";
import { GIT_HOST_KINDS, GIT_HOSTS } from "./gitHosts";
import { SignedInCard } from "./SignedInCard";
import { TokenSignInCard } from "./TokenSignInCard";

function GitHostCard({
	host,
	signedIn,
}: {
	host: HostKindView;
	signedIn: SignedInHostView | null;
}) {
	if (signedIn) return <SignedInCard signedIn={signedIn} />;
	if (GIT_HOSTS[host].signInMethod === "browser") return <BrowserSignInCard host={host} />;
	return <TokenSignInCard host={host} />;
}

/** Settings → Integrations: sign in to and out of each Git host. */
export function GitHostingSettings() {
	const { t } = useTranslation();
	const { findSignedInHost, loading, error } = useHostedAccounts();

	return (
		<Stack gap="sm">
			<Title order={3}>{t("settings.gitProvider.title")}</Title>
			<Text size="sm" c="dimmed">
				{t("settings.gitProvider.description")}
			</Text>
			{error && (
				<Alert color="red" title={t("settings.gitProvider.loadError")}>
					{describeError(error)}
				</Alert>
			)}
			{!loading &&
				!error &&
				GIT_HOST_KINDS.map((host) => (
					<GitHostCard key={host} host={host} signedIn={findSignedInHost(host)} />
				))}
		</Stack>
	);
}
