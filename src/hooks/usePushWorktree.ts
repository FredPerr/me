import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";

/** Mirrors the Rust `PushResult` returned by the `push_worktree` command. */
export type PushResult = {
	path: string;
	pushed: boolean;
	message: string | null;
};

/**
 * Pushes a single worktree's current branch to its upstream. The backend
 * injects the stored provider access token for HTTPS GitHub remotes, so no
 * credentials are handled here.
 */
export function usePushWorktree() {
	const [pushing, setPushing] = useState(false);

	const push = useCallback(async (worktreePath: string): Promise<PushResult> => {
		setPushing(true);
		try {
			return await invoke<PushResult>("push_worktree", { path: worktreePath });
		} finally {
			setPushing(false);
		}
	}, []);

	return { push, pushing };
}
