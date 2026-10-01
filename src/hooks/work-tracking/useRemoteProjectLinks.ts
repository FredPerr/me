import { notifications } from "@mantine/notifications";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { linkRemoteProject } from "@/application/work-tracking/linkRemoteProject";
import { unlinkRemoteProject } from "@/application/work-tracking/unlinkRemoteProject";
import { translateError } from "@/components/work-tracking/translateError";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { RemoteProjectLink } from "@/domain/work-tracking/RemoteProjectLink";
import type { WorkProject } from "@/domain/work-tracking/WorkProject";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { useProjects } from "@/hooks/useProjects";
import { remoteProjectLinkRepository } from "@/infra/work-tracking/workTracking";
import type { Project } from "@/models/Project";

export type RemoteProjectLinkActions = {
	projects: Project[];
	loading: boolean;
	link: (project: Project, remoteProject: WorkProject, connection: ProviderConnection) => void;
	unlink: (project: Project, link: RemoteProjectLink) => void;
	isPending: (projectTag: string, identityKey: string) => boolean;
};

function pendingKeyOf(projectTag: string, identityKey: string): string {
	return `${projectTag}|${identityKey}`;
}

export function useRemoteProjectLinks(): RemoteProjectLinkActions {
	const { t } = useTranslation();
	const { projects, loading, reload } = useProjects();
	const [pendingKeys, setPendingKeys] = useState<ReadonlySet<string>>(() => new Set());

	const runOperation = useCallback(
		async (pendingKey: string, fallbackMessageKey: string, operation: () => Promise<void>) => {
			setPendingKeys((current) => new Set(current).add(pendingKey));
			try {
				await operation();
				await reload();
			} catch (error) {
				notifications.show({
					title: t("common.error"),
					message:
						error instanceof WorkTrackingError ? translateError(t, error) : t(fallbackMessageKey),
					color: "red",
				});
			} finally {
				setPendingKeys((current) => {
					const next = new Set(current);
					next.delete(pendingKey);
					return next;
				});
			}
		},
		[reload, t],
	);

	const link = useCallback(
		(project: Project, remoteProject: WorkProject, connection: ProviderConnection) => {
			const identityKey = RemoteProjectLink.identityKeyOf(
				connection.kind,
				connection.id,
				remoteProject.id,
			);
			void runOperation(pendingKeyOf(project.tag, identityKey), "workTracking.link.linkError", () =>
				linkRemoteProject(remoteProjectLinkRepository, {
					projectTag: project.tag,
					providerKind: connection.kind,
					connectionId: connection.id,
					remoteProjectId: remoteProject.id,
					remoteProjectName: remoteProject.name,
					remoteProjectUrl: remoteProject.url,
				}),
			);
		},
		[runOperation],
	);

	const unlink = useCallback(
		(project: Project, remoteProjectLink: RemoteProjectLink) => {
			const identityKey = remoteProjectLink.identityKey();
			void runOperation(
				pendingKeyOf(project.tag, identityKey),
				"workTracking.link.unlinkError",
				() =>
					unlinkRemoteProject(remoteProjectLinkRepository, {
						projectTag: project.tag,
						identityKey,
					}),
			);
		},
		[runOperation],
	);

	const isPending = useCallback(
		(projectTag: string, identityKey: string) =>
			pendingKeys.has(pendingKeyOf(projectTag, identityKey)),
		[pendingKeys],
	);

	return { projects, loading, link, unlink, isPending };
}
