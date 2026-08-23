import { invoke } from "@tauri-apps/api/core";
import { useCallback } from "react";
import { useAppSettings } from "@/hooks/useAppSettings";

export function useOpenInIde() {
	const { settings } = useAppSettings();

	const open = useCallback(
		async (projectPath: string) => {
			if (settings?.ideCommand && projectPath) {
				await invoke("open_in_ide", { command: settings.ideCommand, path: projectPath });
			}
		},
		[settings],
	);

	const isAvailable = !!settings?.ideCommand;

	return { open, isAvailable };
}
