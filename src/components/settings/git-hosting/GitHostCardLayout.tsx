import { Avatar, Group, Paper, Stack, Text } from "@mantine/core";
import { GitBranchIcon, GithubLogoIcon } from "@phosphor-icons/react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { HostKindView, SignedInHostView } from "@/bindings";
import { GIT_HOSTS } from "@/models/git-hosting/gitHosts";

const CARD_ICON_SIZE = 28;

export function GitHostIcon({ host, size }: { host: HostKindView; size: number }) {
	if (host === "github") return <GithubLogoIcon size={size} weight="fill" />;
	return <GitBranchIcon size={size} weight="fill" />;
}

type GitHostCardLayoutProps = {
	host: HostKindView;
	signedIn: SignedInHostView | null;
	action: ReactNode;
	children?: ReactNode;
};

/** The frame of a Git host card: who is signed in, one action, and the sign-in step below. */
export function GitHostCardLayout({ host, signedIn, action, children }: GitHostCardLayoutProps) {
	const { t } = useTranslation();
	const account = signedIn?.account ?? null;

	function statusText() {
		if (account) return t("settings.gitHosting.connectedAs", { login: account.login });
		if (signedIn) return t("settings.gitHosting.connectedWithoutAccount");
		return t("settings.gitHosting.notConnected");
	}

	return (
		<Paper withBorder p="md" radius="md">
			<Group justify="space-between" wrap="nowrap">
				<Group gap="sm" wrap="nowrap">
					{account?.avatarUrl ? (
						<Avatar src={account.avatarUrl} radius="xl" size="md" alt={account.login} />
					) : (
						<GitHostIcon host={host} size={CARD_ICON_SIZE} />
					)}
					<Stack gap={0}>
						<Text fw={500}>{t(GIT_HOSTS[host].displayNameKey)}</Text>
						<Text size="sm" c="dimmed">
							{statusText()}
						</Text>
					</Stack>
				</Group>
				{action}
			</Group>
			{children}
		</Paper>
	);
}
