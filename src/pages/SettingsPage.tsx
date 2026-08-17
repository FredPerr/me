import { Container, Stack, Title } from "@mantine/core";
import { ProjectSettings } from "@/components/settings/ProjectSettings";

export function SettingsPage() {
	return (
		<Container size="sm" py="xl">
			<Stack gap="xl">
				<Title order={2}>Settings</Title>
				<ProjectSettings />
			</Stack>
		</Container>
	);
}
