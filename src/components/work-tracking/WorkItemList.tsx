import { Stack, Title } from "@mantine/core";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { WorkItemId } from "@/domain/work-tracking/identifiers";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import type { WorkItem } from "@/domain/work-tracking/WorkItem";
import { buildWorkItemTree, type WorkItemNode } from "@/domain/work-tracking/WorkItemTree";
import type { PagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";
import { LoadMoreButton } from "./LoadMoreButton";
import { RemoteListState } from "./RemoteListState";
import { WorkItemRow } from "./WorkItemRow";

type WorkItemSelection = {
	selectedIds: ReadonlySet<WorkItemId>;
	onToggle: (item: WorkItem, selected: boolean) => void;
};

type WorkItemListProps = {
	list: PagedRemoteList<WorkItem>;
	connection: ProviderConnection;
	onReplaceKey: () => void;
	selection?: WorkItemSelection;
};

export function WorkItemList({ list, connection, onReplaceKey, selection }: WorkItemListProps) {
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
				<WorkItemNodes nodes={roots} connection={connection} depth={0} selection={selection} />
				<LoadMoreButton list={list} />
			</RemoteListState>
		</Stack>
	);
}

type WorkItemNodesProps = {
	nodes: readonly WorkItemNode[];
	connection: ProviderConnection;
	depth: number;
	selection?: WorkItemSelection;
};

function WorkItemNodes({ nodes, connection, depth, selection }: WorkItemNodesProps) {
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
						selectable={selection !== undefined}
						selected={selection?.selectedIds.has(node.item.id) ?? false}
						onToggleSelected={selection?.onToggle}
					/>
					{node.children.length > 0 && (
						<WorkItemNodes
							nodes={node.children}
							connection={connection}
							depth={depth + 1}
							selection={selection}
						/>
					)}
				</li>
			))}
		</ul>
	);
}
