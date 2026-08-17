import { Button, Group, Stack, TextInput, Title } from "@mantine/core";
import { useState, useEffect } from "react";
import { useAppSettings } from "@/hooks/useAppSettings";

export function GeneralSettings() {
	const { settings, loading, save } = useAppSettings();
	const [ideCommand, setIdeCommand] = useState("");

	useEffect(() => {
		if (settings) {
			setIdeCommand(settings.ideCommand);
		}
	}, [settings]);

	async function handleSave() {
		await save({ ...settings!, ideCommand });
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
			<Group justify="flex-end">
				<Button onClick={handleSave}>Save</Button>
			</Group>
		</Stack>
	);
}
