import { notifications } from "@mantine/notifications";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Project } from "@/models/Project";

type PullResult = {
	path: string;
	pulled: boolean;
	message: string | null;
};

function collectWorktreePaths(project: Project): string[] {
	const paths = new Set<string>();

	for (const context of project.contexts) {
		for (const branch of context.branches) {
			const repository = project.findRepository(branch.repositoryId);
			if (!repository) continue;
			paths.add(context.getWorktreePath(project.path, repository));
		}
	}

	return [...paths];
}

export function usePullContexts(project: Project) {
	const { t } = useTranslation();
	const [pulling, setPulling] = useState(false);

	const pullAll = useCallback(async () => {
		setPulling(true);
		try {
			const paths = collectWorktreePaths(project);
			const results = await Promise.allSettled(
				paths.map((path) => invoke<PullResult>("pull_worktree", { path })),
			);

			const pulledCount = results.filter(
				(result) => result.status === "fulfilled" && result.value.pulled,
			).length;
			const failures = results.filter((result) => result.status === "rejected");

			if (failures.length > 0) {
				notifications.show({
					title: t("project.pullFailedTitle"),
					message: t("project.pullFailedMessage", { count: failures.length }),
					color: "red",
				});
				return;
			}

			notifications.show({
				title: t("project.pullDoneTitle"),
				message: t("project.pullDoneMessage", { count: pulledCount }),
				color: "green",
			});
		} finally {
			setPulling(false);
		}
	}, [project, t]);

	return { pullAll, pulling };
}
