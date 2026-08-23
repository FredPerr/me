import { Button, Group, NumberInput, Stack, Switch, TextInput, Title } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppSettings } from "@/hooks/useAppSettings";

export function GeneralSettings() {
	const { t } = useTranslation();
	const { settings, loading, save } = useAppSettings();
	const [ideCommand, setIdeCommand] = useState("");
	const [autoRefreshEnabled, setAutoRefreshEnabled] = useState(true);
	const [refreshInterval, setRefreshInterval] = useState<number>(5);

	useEffect(() => {
		if (settings) {
			setIdeCommand(settings.ideCommand);
			setAutoRefreshEnabled(settings.workspaceRefreshInterval !== null);
			setRefreshInterval(settings.workspaceRefreshInterval ?? 5);
		}
	}, [settings]);

	async function handleSave() {
		if (!settings) return;
		try {
			await save({
				...settings,
				ideCommand,
				workspaceRefreshInterval: autoRefreshEnabled ? refreshInterval : null,
			});
			notifications.show({
				title: t("settings.general.saved"),
				message: t("settings.general.savedMessage"),
				color: "green",
			});
		} catch {
			notifications.show({
				title: t("common.error"),
				message: t("settings.general.saveError"),
				color: "red",
			});
		}
	}

	if (loading) {
		return null;
	}

	return (
		<Stack gap="sm">
			<Title order={3}>{t("settings.general.title")}</Title>
			<TextInput
				label={t("settings.general.ideCommand")}
				description={t("settings.general.ideCommandDescription")}
				value={ideCommand}
				onChange={(event) => setIdeCommand(event.currentTarget.value)}
			/>
			<Switch
				label={t("settings.general.autoRefresh")}
				checked={autoRefreshEnabled}
				onChange={(event) => setAutoRefreshEnabled(event.currentTarget.checked)}
			/>
			{autoRefreshEnabled && (
				<NumberInput
					label={t("settings.general.refreshInterval")}
					value={refreshInterval}
					onChange={(value) => setRefreshInterval(Number(value) || 5)}
					min={1}
					max={60}
				/>
			)}
			<Group justify="flex-end">
				<Button onClick={handleSave}>{t("common.save")}</Button>
			</Group>
		</Stack>
	);
}
