import { Button, Group, NumberInput, Stack, Switch, TextInput, Title } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useEffect, useState } from "react";
import { useAppSettings } from "@/hooks/useAppSettings";

export function GeneralSettings() {
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
				title: "Settings saved",
				message: "Your settings have been saved successfully.",
				color: "green",
			});
		} catch {
			notifications.show({
				title: "Error",
				message: "Failed to save settings.",
				color: "red",
			});
		}
	}

	if (loading) {
		return null;
	}

	return (
		<Stack gap="sm">
			<Title order={3}>General</Title>
			<TextInput
				label="IDE Command"
				description="Command used to open projects (e.g. kiro, code, cursor)"
				value={ideCommand}
				onChange={(event) => setIdeCommand(event.currentTarget.value)}
			/>
			<Switch
				label="Auto-refresh active workspaces"
				checked={autoRefreshEnabled}
				onChange={(event) => setAutoRefreshEnabled(event.currentTarget.checked)}
			/>
			{autoRefreshEnabled && (
				<NumberInput
					label="Refresh interval (seconds)"
					value={refreshInterval}
					onChange={(value) => setRefreshInterval(Number(value) || 5)}
					min={1}
					max={60}
				/>
			)}
			<Group justify="flex-end">
				<Button onClick={handleSave}>Save</Button>
			</Group>
		</Stack>
	);
}
