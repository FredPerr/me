import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export type Worktree = {
	name: string;
	path: string;
	branch: string | null;
	is_default: boolean;
};

export function useWorktrees(projectPath: string | undefined) {
	const [worktrees, setWorktrees] = useState<Worktree[]>([]);

	const refresh = useCallback(async () => {
		if (!projectPath) {
			setWorktrees([]);
			return;
		}

		try {
			const result = await invoke<Worktree[]>("list_worktrees", { path: projectPath });
			setWorktrees(result);
		} catch {
			setWorktrees([]);
		}
	}, [projectPath]);

	useEffect(() => {
		refresh();
	}, [refresh]);

	return { worktrees, refresh };
}
