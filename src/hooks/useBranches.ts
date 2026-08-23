import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export function useBranches(repositoryPath: string | undefined) {
	const [branches, setBranches] = useState<string[]>([]);

	const refresh = useCallback(async () => {
		if (!repositoryPath) {
			setBranches([]);
			return;
		}

		try {
			const result = await invoke<string[]>("list_branches", { path: repositoryPath });
			setBranches(result);
		} catch {
			setBranches([]);
		}
	}, [repositoryPath]);

	useEffect(() => {
		refresh();
	}, [refresh]);

	return { branches, refresh };
}
