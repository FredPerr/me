import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export type WorktreeStatus = {
	files_added: number;
	files_modified: number;
	files_deleted: number;
	files_renamed: number;
	insertions: number;
	deletions: number;
	pr_url: string | null;
};

export function useWorktreeStatus(path: string | undefined) {
	const [status, setStatus] = useState<WorktreeStatus | null>(null);

	const refresh = useCallback(async () => {
		if (!path) {
			setStatus(null);
			return;
		}

		try {
			const result = await invoke<WorktreeStatus>("get_worktree_status", { path });
			setStatus(result);
		} catch {
			setStatus(null);
		}
	}, [path]);

	useEffect(() => {
		refresh();
	}, [refresh]);

	return { status, refresh };
}
