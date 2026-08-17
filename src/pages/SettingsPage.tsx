import { Container, Divider, Stack, Title } from "@mantine/core";
import { GeneralSettings } from "@/components/settings/GeneralSettings";
import { ProjectSettings } from "@/components/settings/ProjectSettings";

export function SettingsPage() {
	return (
		<Container size="sm" py="xl">
			<Stack gap="xl">
				<Title order={2}>Settings</Title>
				<GeneralSettings />
				<Divider />
				<ProjectSettings />
			</Stack>
		</Container>
	);
}
