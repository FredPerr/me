import { useCallback, useEffect, useState } from "react";
import { type AppSettings, AppSettingsRepository } from "@/models/AppSettings";

export function useAppSettings() {
	const [settings, setSettings] = useState<AppSettings | null>(null);
	const [loading, setLoading] = useState(true);

	const reload = useCallback(async () => {
		const loaded = await AppSettingsRepository.load();
		setSettings(loaded);
		setLoading(false);
	}, []);

	useEffect(() => {
		reload();
	}, [reload]);

	const save = useCallback(async (updated: AppSettings) => {
		await AppSettingsRepository.save(updated);
		setSettings(updated);
	}, []);

	return { settings, loading, save, reload };
}
