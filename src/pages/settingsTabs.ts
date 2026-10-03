export enum SettingsTab {
	General = "general",
	Integrations = "integrations",
	Projects = "projects",
}

export const SETTINGS_TAB_PARAM = "tab";

export function isSettingsTab(value: string | null): value is SettingsTab {
	return Object.values<string | null>(SettingsTab).includes(value);
}

export function settingsPath(tab: SettingsTab): string {
	return `/settings?${SETTINGS_TAB_PARAM}=${tab}`;
}
