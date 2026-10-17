import { Stack, Title } from "@mantine/core";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ConnectionView, Task, TaskId } from "@/bindings";
import { buildTaskTree, type TaskNode } from "@/hooks/work-tracking/taskTree";
import type { PagedRemoteList } from "@/hooks/work-tracking/usePagedRemoteList";
import { LoadMoreButton } from "./LoadMoreButton";
import { RemoteListState } from "./RemoteListState";
import { WorkItemRow } from "./WorkItemRow";

type WorkItemSelection = {
	selectedIds: ReadonlySet<TaskId>;
	onToggle: (task: Task, selected: boolean) => void;
};

type WorkItemListProps = {
	list: PagedRemoteList<Task>;
	connection: ConnectionView;
	onReplaceKey: () => void;
	selection?: WorkItemSelection;
};

export function WorkItemList({ list, connection, onReplaceKey, selection }: WorkItemListProps) {
	const { t } = useTranslation();
	const roots = useMemo(() => buildTaskTree(list.items), [list.items]);

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
	nodes: readonly TaskNode[];
	connection: ConnectionView;
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
				<li key={node.task.id}>
					<WorkItemRow
						task={node.task}
						connection={connection}
						isOrphanSubtask={depth === 0 && node.task.parentId !== null}
						selectable={selection !== undefined}
						selected={selection?.selectedIds.has(node.task.id) ?? false}
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
