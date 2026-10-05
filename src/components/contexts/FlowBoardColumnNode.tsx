import { Handle, Position } from "@xyflow/react";
import type { PullRequestLookup } from "@/hooks/useContextPullRequestStatus";
import type { CreateContextParams } from "@/hooks/useContexts";
import { type ContextStatus, IDLE_CONTEXT_STATUS } from "@/models/ContextStatus";
import type { Context, Project } from "@/models/Project";
import { CreateFromTasksColumnButton } from "./CreateFromTasksColumnButton";
import { DraggableContextCard } from "./DraggableContextCard";
import { KanbanColumn } from "./KanbanColumn";

export type FlowBoardColumnData = {
	status: ContextStatus;
	contexts: Context[];
	project: Project;
	allContexts: Context[];
	pullRequestLookup?: PullRequestLookup;
	onDelete: (contextId: string) => void;
	onCreate: (params: CreateContextParams) => Promise<void>;
	onExpandedChange: (contextId: string, expanded: boolean) => void;
	onStaticChange: (contextId: string, isStatic: boolean) => void;
	onOpenKiro: (context: Context) => void;
	isKiroRunning: (contextId: string) => boolean;
	getKiroHistoryCount: (contextId: string) => number;
	onCreateFromTasks?: () => void;
};

export function FlowBoardColumnNode({ data }: { data: FlowBoardColumnData }) {
	const {
		status,
		contexts,
		project,
		allContexts,
		pullRequestLookup,
		onDelete,
		onCreate,
		onExpandedChange,
		onStaticChange,
		onOpenKiro,
		isKiroRunning,
		getKiroHistoryCount,
		onCreateFromTasks,
	} = data;

	return (
		<div
			className="nodrag nopan nowheel"
			style={{
				width: 340,
				minHeight: 150,
				display: "flex",
				userSelect: "text",
				cursor: "auto",
			}}
		>
			<Handle type="target" position={Position.Left} style={{ opacity: 0 }} isConnectable={false} />
			<KanbanColumn
				status={status}
				count={contexts.length}
				headerAction={
					status === IDLE_CONTEXT_STATUS && onCreateFromTasks ? (
						<CreateFromTasksColumnButton onClick={onCreateFromTasks} />
					) : undefined
				}
			>
				{contexts.map((context) => (
					<DraggableContextCard
						key={context.id}
						context={context}
						project={project}
						allContexts={allContexts}
						onDelete={onDelete}
						onCreate={onCreate}
						pullRequestLookup={pullRequestLookup}
						onExpandedChange={onExpandedChange}
						onStaticChange={onStaticChange}
						onOpenKiro={onOpenKiro}
						isKiroRunning={isKiroRunning(context.id)}
						hasKiroHistory={getKiroHistoryCount(context.id) > 0}
					/>
				))}
			</KanbanColumn>
			<Handle
				type="source"
				position={Position.Right}
				style={{ opacity: 0 }}
				isConnectable={false}
			/>
		</div>
	);
}
