import { Container, Divider, Stack, Title } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { GeneralSettings } from "@/components/settings/GeneralSettings";
import { GitProviderSettings } from "@/components/settings/GitProviderSettings";
import { ProjectSettings } from "@/components/settings/ProjectSettings";

export function SettingsPage() {
	const { t } = useTranslation();
	return (
		<Container size="sm" py="xl">
			<Stack gap="xl">
				<Title order={2}>{t("settings.title")}</Title>
				<GeneralSettings />
				<Divider />
				<GitProviderSettings />
				<Divider />
				<ProjectSettings />
			</Stack>
		</Container>
	);
}
