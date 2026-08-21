import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { useAppSettings } from "@/hooks/useAppSettings";

export function useActiveWorkspaces() {
	const { settings } = useAppSettings();
	const [activeWorkspaces, setActiveWorkspaces] = useState<string[]>([]);
	const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

	const refresh = useCallback(async () => {
		if (!settings?.ideCommand) {
			setActiveWorkspaces([]);
			return;
		}

		const workspaces = await invoke<string[]>("get_active_workspaces", {
			ideCommand: settings.ideCommand,
		});
		setActiveWorkspaces(workspaces);
	}, [settings?.ideCommand]);

	useEffect(() => {
		refresh();
	}, [refresh]);

	useEffect(() => {
		if (intervalRef.current) {
			clearInterval(intervalRef.current);
			intervalRef.current = null;
		}

		const intervalSeconds = settings?.workspaceRefreshInterval;
		if (intervalSeconds && intervalSeconds > 0) {
			intervalRef.current = setInterval(refresh, intervalSeconds * 1000);
		}

		return () => {
			if (intervalRef.current) {
				clearInterval(intervalRef.current);
			}
		};
	}, [settings?.workspaceRefreshInterval, refresh]);

	function isActive(path: string): boolean {
		const normalized = path.endsWith("/") ? path.slice(0, -1) : path;
		return activeWorkspaces.some((workspace) => {
			const normalizedWorkspace = workspace.endsWith("/") ? workspace.slice(0, -1) : workspace;
			return normalizedWorkspace === normalized;
		});
	}

	return { activeWorkspaces, isActive, refresh };
}
