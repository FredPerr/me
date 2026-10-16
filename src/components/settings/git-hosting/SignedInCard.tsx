import { Button } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useTranslation } from "react-i18next";
import type { SignedInHostView } from "@/bindings";
import { useSignOut } from "@/hooks/gitHosting/useGitHostingMutations";
import { GitHostCardLayout } from "./GitHostCardLayout";
import { GIT_HOSTS } from "./gitHosts";

/** A signed-in Git host. A failed sign-out is shown by the app-wide notification. */
export function SignedInCard({ signedIn }: { signedIn: SignedInHostView }) {
	const { t } = useTranslation();
	const signOut = useSignOut();

	function handleSignOut() {
		signOut.mutate(signedIn.host, {
			onSuccess: () =>
				notifications.show({
					title: t("settings.gitProvider.disconnectedTitle"),
					message: t("settings.gitProvider.disconnectedMessage", {
						provider: t(GIT_HOSTS[signedIn.host].displayNameKey),
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
					{t("settings.gitProvider.disconnect")}
				</Button>
			}
		/>
	);
}
