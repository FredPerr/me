import { notifications } from "@mantine/notifications";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type ProjectView } from "@/bindings";
import { useErrorModal } from "@/components/shared/ErrorModalProvider";
import { describeError } from "@/infra/commandError";
import { collectWorktreePaths } from "@/models/projectView";

function describeFailure(path: string, reason: unknown): string {
	const message = describeError(reason);
	return `• ${path}: ${message}`;
}

export function usePullContexts(project: ProjectView) {
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
