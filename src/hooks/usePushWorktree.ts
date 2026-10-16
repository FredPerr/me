import { useCallback, useState } from "react";
import { commands, type PushResult } from "@/bindings";

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
			return await commands.pushWorktree(worktreePath);
		} finally {
			setPushing(false);
		}
	}, []);

	return { push, pushing };
}
