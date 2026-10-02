import { Stack, Text, Title } from "@mantine/core";
import { useTranslation } from "react-i18next";
import { listGitProviders } from "@/models/git-provider/GitProviderRegistry";
import { GitProviderCard } from "./GitProviderCard";

export function GitProviderSettings() {
	const { t } = useTranslation();
	const providers = listGitProviders();

	return (
		<Stack gap="sm">
			<Title order={3}>{t("settings.gitProvider.title")}</Title>
			<Text size="sm" c="dimmed">
				{t("settings.gitProvider.description")}
			</Text>

			{providers.map((provider) => (
				<GitProviderCard key={provider.id} provider={provider} />
			))}
		</Stack>
	);
}
