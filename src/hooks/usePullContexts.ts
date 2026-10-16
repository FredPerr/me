import { notifications } from "@mantine/notifications";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { useErrorModal } from "@/components/shared/ErrorModalProvider";
import { describeError } from "@/infra/commandError";
import type { Project } from "@/models/Project";

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

function describeFailure(path: string, reason: unknown): string {
	const message = describeError(reason);
	return `• ${path}: ${message}`;
}

export function usePullContexts(project: Project) {
	const { t } = useTranslation();
	const { showError } = useErrorModal();
	const [pulling, setPulling] = useState(false);

	const pullAll = useCallback(async () => {
		setPulling(true);
		try {
			const paths = collectWorktreePaths(project);
			const results = await Promise.allSettled(paths.map((path) => commands.pullWorktree(path)));

			const pulledCount = results.filter(
				(result) => result.status === "fulfilled" && result.value.pulled,
			).length;
			const failureReasons = results.flatMap((result, index) =>
				result.status === "rejected" ? [describeFailure(paths[index], result.reason)] : [],
			);

			if (failureReasons.length > 0) {
				showError({
					title: t("project.pullFailedTitle"),
					description: t("project.pullFailedMessage", { count: failureReasons.length }),
					details: failureReasons.join("\n"),
					hint: t("project.pullFailedHint"),
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
	}, [project, t, showError]);

	return { pullAll, pulling };
}
