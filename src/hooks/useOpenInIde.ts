import { useCallback } from "react";
import { useAppSettings } from "@/hooks/useAppSettings";
import { openInIde } from "@/utils/openInIde";

export function useOpenInIde() {
	const { settings } = useAppSettings();

	const open = useCallback(
		async (projectPath: string) => {
			if (settings?.ideCommand && projectPath) {
				await openInIde(settings.ideCommand, projectPath);
			}
		},
		[settings],
	);

	const isAvailable = !!settings?.ideCommand;

	return { open, isAvailable };
}
