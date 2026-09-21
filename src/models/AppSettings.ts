import { load } from "@tauri-apps/plugin-store";

export type AppSettings = {
	ideCommand: string;
	shell: string | null;
	workspaceRefreshInterval: number | null;
};

const SETTINGS_STORE_FILE = "settings.json";
const SETTINGS_KEY = "settings";

const DEFAULT_SETTINGS: AppSettings = {
	ideCommand: "kiro",
	shell: null,
	workspaceRefreshInterval: 5,
};

async function openStore() {
	return load(SETTINGS_STORE_FILE, { autoSave: true });
}

export const AppSettingsRepository = {
	async load(): Promise<AppSettings> {
		const store = await openStore();
		const stored = await store.get<Partial<AppSettings>>(SETTINGS_KEY);

		return { ...DEFAULT_SETTINGS, ...stored };
	},

	async save(settings: AppSettings): Promise<void> {
		const store = await openStore();
		await store.set(SETTINGS_KEY, settings);
		await store.save();
	},
};
