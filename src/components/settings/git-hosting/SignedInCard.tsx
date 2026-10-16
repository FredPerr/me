import { Button } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useTranslation } from "react-i18next";
import type { SignedInHostView } from "@/bindings";
import { useSignOut } from "@/hooks/gitHosting/useGitHostingMutations";
import { GIT_HOSTS } from "@/models/git-hosting/gitHosts";
import { GitHostCardLayout } from "./GitHostCardLayout";

/** A signed-in Git host. A failed sign-out is shown by the app-wide notification. */
export function SignedInCard({ signedIn }: { signedIn: SignedInHostView }) {
	const { t } = useTranslation();
	const signOut = useSignOut();

	function handleSignOut() {
		signOut.mutate(signedIn.host, {
			onSuccess: () =>
				notifications.show({
					title: t("settings.gitHosting.disconnectedTitle"),
					message: t("settings.gitHosting.disconnectedMessage", {
						host: t(GIT_HOSTS[signedIn.host].displayNameKey),
					}),
					color: "green",
				}),
		});
	}

	return (
		<GitHostCardLayout
			host={signedIn.host}
			signedIn={signedIn}
			action={
				<Button variant="light" color="red" onClick={handleSignOut} loading={signOut.isPending}>
					{t("settings.gitHosting.disconnect")}
				</Button>
			}
		/>
	);
}
