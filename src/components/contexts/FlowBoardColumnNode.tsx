import { Handle, Position } from "@xyflow/react";
import type { ContextView, ProjectView } from "@/bindings";
import { type ContextStatus, IDLE_CONTEXT_STATUS } from "@/models/ContextStatus";
import type { PullRequestLookup } from "@/models/git-hosting/branchPullRequests";
import { CreateFromTasksColumnButton } from "./CreateFromTasksColumnButton";
import { DraggableContextCard } from "./DraggableContextCard";
import { KanbanColumn } from "./KanbanColumn";

export type FlowBoardColumnData = {
	status: ContextStatus;
	contexts: ContextView[];
	project: ProjectView;
	pullRequestLookup?: PullRequestLookup;
	onOpenKiro: (context: ContextView) => void;
	isKiroRunning: (contextId: string) => boolean;
	hasKiroHistory: (contextId: string) => boolean;
	onCreateFromTasks?: () => void;
};

export function FlowBoardColumnNode({ data }: { data: FlowBoardColumnData }) {
	const {
		status,
		contexts,
		project,
		pullRequestLookup,
		onOpenKiro,
		isKiroRunning,
		hasKiroHistory,
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
						pullRequestLookup={pullRequestLookup}
						onOpenKiro={onOpenKiro}
						isKiroRunning={isKiroRunning(context.id)}
						hasKiroHistory={hasKiroHistory(context.id)}
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
