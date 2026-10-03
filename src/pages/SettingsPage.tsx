import { Container, Divider, Stack, Tabs, Title } from "@mantine/core";
import { FolderIcon, GearIcon, PlugsConnectedIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
import { GeneralSettings } from "@/components/settings/GeneralSettings";
import { GitProviderSettings } from "@/components/settings/GitProviderSettings";
import { ProjectSettings } from "@/components/settings/ProjectSettings";
import { WorkTrackerSettings } from "@/components/settings/WorkTrackerSettings";
import { isSettingsTab, SETTINGS_TAB_PARAM, SettingsTab } from "./settingsTabs";

export function SettingsPage() {
	const { t } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();
	const requestedTab = searchParams.get(SETTINGS_TAB_PARAM);
	const activeTab = isSettingsTab(requestedTab) ? requestedTab : SettingsTab.General;

	function handleTabChange(tab: string | null) {
		if (isSettingsTab(tab)) setSearchParams({ [SETTINGS_TAB_PARAM]: tab }, { replace: true });
	}

	return (
		<Container fluid w="100%" p="lg">
			<Stack gap="xl">
				<Title order={2}>{t("settings.title")}</Title>
				<Tabs value={activeTab} onChange={handleTabChange}>
					<Tabs.List grow>
						<Tabs.Tab value={SettingsTab.General} leftSection={<GearIcon size={16} />}>
							{t("settings.tabs.general")}
						</Tabs.Tab>
						<Tabs.Tab
							value={SettingsTab.Integrations}
							leftSection={<PlugsConnectedIcon size={16} />}
						>
							{t("settings.tabs.integrations")}
						</Tabs.Tab>
						<Tabs.Tab value={SettingsTab.Projects} leftSection={<FolderIcon size={16} />}>
							{t("settings.tabs.projects")}
						</Tabs.Tab>
					</Tabs.List>
					<Tabs.Panel value={SettingsTab.General} pt="lg">
						<GeneralSettings />
					</Tabs.Panel>
					<Tabs.Panel value={SettingsTab.Integrations} pt="lg">
						<Stack gap="xl">
							<GitProviderSettings />
							<Divider />
							<WorkTrackerSettings />
						</Stack>
					</Tabs.Panel>
					<Tabs.Panel value={SettingsTab.Projects} pt="lg">
						<ProjectSettings />
					</Tabs.Panel>
				</Tabs>
			</Stack>
		</Container>
	);
}
