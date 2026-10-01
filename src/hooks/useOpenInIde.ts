import { invoke } from "@tauri-apps/api/core";
import { useCallback } from "react";
import { useAppSettings } from "@/hooks/useAppSettings";

export function useOpenInIde() {
	const { settings } = useAppSettings();

	const open = useCallback(
		async (projectPath: string) => {
			if (settings?.ideCommand && projectPath) {
				await invoke("open_in_ide", {
					command: settings.ideCommand,
					path: projectPath,
					shell: settings.shell,
				});
			}
		},
		[settings],
	);

	const openFirstExisting = useCallback(
		async (candidatePaths: string[]) => {
			if (candidatePaths.length === 0 || !settings?.ideCommand) return;
			for (const candidatePath of candidatePaths) {
				const pathExists = await invoke<boolean>("check_path_exists", { path: candidatePath });
				if (pathExists) {
					await open(candidatePath);
					return;
				}
			}
			await open(candidatePaths[candidatePaths.length - 1]);
		},
		[open, settings],
	);

	const isAvailable = !!settings?.ideCommand;

	return { open, openFirstExisting, isAvailable };
}
