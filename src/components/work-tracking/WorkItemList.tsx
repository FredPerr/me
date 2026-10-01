import { Stack, Title } from "@mantine/core";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import { buildWorkItemTree, type WorkItemNode } from "@/domain/work-tracking/WorkItemTree";
import type { PagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";
import { LoadMoreButton } from "./LoadMoreButton";
import { RemoteListState } from "./RemoteListState";
import { WorkItemRow } from "./WorkItemRow";

type WorkItemListProps = {
	list: PagedRemoteList<WorkItem>;
	connection: ProviderConnection;
	onReplaceKey: () => void;
};

export function WorkItemList({ list, connection, onReplaceKey }: WorkItemListProps) {
	const { t } = useTranslation();
	const roots = useMemo(() => buildWorkItemTree(list.items), [list.items]);

	return (
		<Stack gap="xs">
			<Title order={3}>{t("workTracking.items.title")}</Title>
			<RemoteListState
				status={list.status}
				error={list.error}
				itemCount={list.items.length}
				emptyText={t("workTracking.items.empty")}
				onRetry={list.retry}
				onReplaceKey={onReplaceKey}
			>
				<WorkItemNodes nodes={roots} connection={connection} depth={0} />
				<LoadMoreButton list={list} />
			</RemoteListState>
		</Stack>
	);
}

type WorkItemNodesProps = {
	nodes: readonly WorkItemNode[];
	connection: ProviderConnection;
	depth: number;
};

function WorkItemNodes({ nodes, connection, depth }: WorkItemNodesProps) {
	return (
		<ul
			// biome-ignore lint/a11y/noRedundantRoles: list-style none drops the implicit list role in WebKit
			role="list"
			style={{ listStyle: "none", margin: 0, padding: 0, paddingLeft: depth > 0 ? 20 : 0 }}
		>
			{nodes.map((node) => (
				<li key={node.item.id}>
					<WorkItemRow
						item={node.item}
						connection={connection}
						isOrphanSubtask={depth === 0 && node.item.parentId !== undefined}
					/>
					{node.children.length > 0 && (
						<WorkItemNodes nodes={node.children} connection={connection} depth={depth + 1} />
					)}
				</li>
			))}
		</ul>
	);
}
