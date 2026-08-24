import {
	ActionIcon,
	Button,
	Group,
	NumberInput,
	Stack,
	Switch,
	TextInput,
	Title,
	Tooltip,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { FolderOpenIcon } from "@phosphor-icons/react";
import { appDataDir } from "@tauri-apps/api/path";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppSettings } from "@/hooks/useAppSettings";

export function GeneralSettings() {
	const { t } = useTranslation();
	const { settings, loading, save } = useAppSettings();
	const [ideCommand, setIdeCommand] = useState("");
	const [autoRefreshEnabled, setAutoRefreshEnabled] = useState(true);
	const [refreshInterval, setRefreshInterval] = useState<number>(5);
	const [settingsFilePath, setSettingsFilePath] = useState("");

	useEffect(() => {
		async function resolveSettingsPath() {
			const dataDir = await appDataDir();
			setSettingsFilePath(dataDir);
		}
		resolveSettingsPath();
	}, []);

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
			<TextInput
				label={t("settings.general.settingsFile")}
				description={t("settings.general.settingsFileDescription")}
				value={settingsFilePath}
				readOnly
				disabled
				rightSection={
					<Tooltip label={t("settings.general.openSettingsFile")}>
						<ActionIcon
							variant="subtle"
							size="sm"
							disabled={!settingsFilePath}
							onClick={() => revealItemInDir(settingsFilePath)}
							aria-label={t("settings.general.openSettingsFile")}
						>
							<FolderOpenIcon size={16} />
						</ActionIcon>
					</Tooltip>
				}
			/>
			<Group justify="flex-end">
				<Button onClick={handleSave}>{t("common.save")}</Button>
			</Group>
		</Stack>
	);
}
