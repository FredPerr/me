import { useCallback, useEffect, useState } from "react";
import { commands } from "@/bindings";

export function useBranches(repositoryPath: string | undefined) {
	const [branches, setBranches] = useState<string[]>([]);

	const refresh = useCallback(async () => {
		if (!repositoryPath) {
			setBranches([]);
			return;
		}

		try {
			const result = await commands.listBranches(repositoryPath);
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
