import { BaseDirectory, exists, readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";

export type AppSettings = {
	ideCommand: string;
	shell: string | null;
	workspaceRefreshInterval: number | null;
};

const SETTINGS_FILE = "settings.json";

const DEFAULT_SETTINGS: AppSettings = {
	ideCommand: "kiro",
	shell: null,
	workspaceRefreshInterval: 5,
};

export const AppSettingsRepository = {
	async load(): Promise<AppSettings> {
		const fileExists = await exists(SETTINGS_FILE, { baseDir: BaseDirectory.AppData });

		if (!fileExists) {
			return DEFAULT_SETTINGS;
		}

		const content = await readTextFile(SETTINGS_FILE, { baseDir: BaseDirectory.AppData });
		return { ...DEFAULT_SETTINGS, ...JSON.parse(content) };
	},

	async save(settings: AppSettings): Promise<void> {
		const content = JSON.stringify(settings, null, 2);
		await writeTextFile(SETTINGS_FILE, content, { baseDir: BaseDirectory.AppData });
	},
};
