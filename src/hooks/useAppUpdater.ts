import { notifications } from "@mantine/notifications";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

type CheckOptions = {
	notifyWhenUpToDate?: boolean;
};

export function useAppUpdater() {
	const { t } = useTranslation();
	const [checking, setChecking] = useState(false);

	const checkForUpdates = useCallback(
		async ({ notifyWhenUpToDate = false }: CheckOptions = {}) => {
			setChecking(true);
			try {
				const update = await check();

				if (!update) {
					if (notifyWhenUpToDate) {
						notifications.show({
							title: t("updater.upToDate"),
							message: t("updater.upToDateMessage"),
							color: "green",
						});
					}
					return;
				}

				notifications.show({
					title: t("updater.available"),
					message: t("updater.availableMessage", { version: update.version }),
					color: "blue",
				});

				await update.downloadAndInstall();

				notifications.show({
					title: t("updater.readyTitle"),
					message: t("updater.readyMessage"),
					color: "green",
				});

				await relaunch();
			} catch (_error) {
				notifications.show({
					title: t("updater.failedTitle"),
					message: t("updater.failedMessage"),
					color: "red",
				});
			} finally {
				setChecking(false);
			}
		},
		[t],
	);

	return { checkForUpdates, checking };
}
