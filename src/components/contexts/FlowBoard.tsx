import {
	closestCorners,
	DndContext,
	type DragEndEvent,
	DragOverlay,
	type DragStartEvent,
	PointerSensor,
	useSensor,
	useSensors,
} from "@dnd-kit/core";
import {
	Background,
	type Edge,
	MarkerType,
	type Node,
	type NodeTypes,
	ReactFlow,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import "./FlowBoard.css";
import {
	GitBranchIcon,
	GitPullRequestIcon,
	ListChecksIcon,
	ShareNetworkIcon,
	SparkleIcon,
} from "@phosphor-icons/react";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ContextView, ProjectView } from "@/bindings";
import type { ContextAgentRunSessions } from "@/hooks/agent-runs/useContextAgentRunSessions";
import { useContextPullRequests } from "@/hooks/gitHosting/useContextPullRequests";
import { useKiroPromptDrafts } from "@/hooks/useKiroPromptDrafts";
import { useSetContextStatus } from "@/hooks/workspace/useContextMutations";
import {
	CONTEXT_STATUSES,
	type ContextStatus,
	IDLE_CONTEXT_STATUS,
	isContextStatus,
} from "@/models/ContextStatus";
import { ContextCard } from "./ContextCard";
import { ContextCardDragHandle } from "./ContextCardDragHandle";
import { type FlowBoardAction, FlowBoardActionsNode } from "./FlowBoardActionsNode";
import { type FlowBoardColumnData, FlowBoardColumnNode } from "./FlowBoardColumnNode";
import { type FlowBoardStaticData, FlowBoardStaticNode } from "./FlowBoardStaticNode";
import { KiroConversationModal } from "./KiroConversationModal";

type FlowBoardProps = Pick<ContextAgentRunSessions, "isRunning" | "liveRunFor"> & {
	project: ProjectView;
	onRunAgent: ContextAgentRunSessions["runAgent"];
	onStopAgent: ContextAgentRunSessions["stopAgentRun"];
	searchFilter?: string;
	onCreateFromTasks?: () => void;
	onBulkCreate: () => void;
	onShareForReview: () => void;
	onCreateFromBranches: () => void;
	onCreateFromPullRequests: () => void;
};

const COLUMN_GAP = 400;
const ACTIONS_ROW_Y = -110;
const STATIC_ROW_Y = -320;
const REVIEW_CONTEXT_STATUS: ContextStatus = "review";

const nodeTypes: NodeTypes = {
	column: FlowBoardColumnNode,
	actions: FlowBoardActionsNode,
	static: FlowBoardStaticNode,
};

function noop() {}

export function FlowBoard({
	project,
	onRunAgent,
	onStopAgent,
	isRunning: isAgentRunning,
	liveRunFor,
	searchFilter,
	onCreateFromTasks,
	onBulkCreate,
	onShareForReview,
	onCreateFromBranches,
	onCreateFromPullRequests,
}: FlowBoardProps) {
	const { t } = useTranslation();
	const { contexts } = project;
	const { mutate: setContextStatus } = useSetContextStatus(project.tag);
	const [activeId, setActiveId] = useState<string | null>(null);
	const { lookup: pullRequestLookup } = useContextPullRequests(project.tag);
	const [openKiroContextId, setOpenKiroContextId] = useState<string | null>(null);
	const { promptFor, changePrompt, keepPrompt } = useKiroPromptDrafts(project.tag);
	const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }));

	const filtered = useMemo(() => {
		const query = searchFilter?.toLowerCase().trim();
		if (!query) return contexts;
		return contexts.filter((c) => c.name.toLowerCase().includes(query));
	}, [contexts, searchFilter]);

	const contextsByStatus = useMemo(() => {
		const groups = new Map<ContextStatus, ContextView[]>(
			CONTEXT_STATUSES.map((status) => [status, []]),
		);
		for (const context of filtered) {
			if (context.isStatic) continue;
			groups.get(context.status)?.push(context);
		}
		return groups;
	}, [filtered]);

	const staticContexts = useMemo(() => filtered.filter((context) => context.isStatic), [filtered]);

	const hasAgentHistory = useCallback(
		(contextId: string) => {
			const context = contexts.find((candidate) => candidate.id === contextId);
			return (context?.agentWork ?? null) !== null || isAgentRunning(contextId);
		},
		[contexts, isAgentRunning],
	);

	const columnIndex = useMemo(
		() => new Map(CONTEXT_STATUSES.map((status, index) => [status, index])),
		[],
	);

	const nodes = useMemo<Node[]>(() => {
		const columnNodes: Node[] = CONTEXT_STATUSES.map((status, index) => ({
			id: status,
			type: "column",
			position: { x: index * COLUMN_GAP, y: 0 },
			draggable: false,
			selectable: false,
			data: {
				status,
				contexts: contextsByStatus.get(status) ?? [],
				project,
				pullRequestLookup,
				onOpenKiro: (target: ContextView) => setOpenKiroContextId(target.id),
				isKiroRunning: isAgentRunning,
				hasKiroHistory: hasAgentHistory,
			} satisfies FlowBoardColumnData,
		}));

		const idleActions: FlowBoardAction[] = [];
		if (onCreateFromTasks) {
			idleActions.push({
				key: "fromTasks",
				label: t("contexts.fromTasks.button"),
				icon: ListChecksIcon,
				onClick: onCreateFromTasks,
			});
		}
		idleActions.push({
			key: "bulk",
			label: t("contexts.bulk.button"),
			icon: SparkleIcon,
			onClick: onBulkCreate,
		});

		const reviewActions: FlowBoardAction[] = [
			{
				key: "share",
				label: t("review.shareButton"),
				icon: ShareNetworkIcon,
				onClick: onShareForReview,
			},
			{
				key: "fromBranches",
				label: t("contexts.createFromBranches"),
				icon: GitBranchIcon,
				onClick: onCreateFromBranches,
			},
			{
				key: "fromPullRequests",
				label: t("contexts.createFromPullRequests"),
				icon: GitPullRequestIcon,
				onClick: onCreateFromPullRequests,
			},
		];

		const actionNodes: Node[] = [
			{
				id: "actions-idle",
				type: "actions",
				position: { x: (columnIndex.get(IDLE_CONTEXT_STATUS) ?? 0) * COLUMN_GAP, y: ACTIONS_ROW_Y },
				draggable: false,
				selectable: false,
				data: { actions: idleActions },
			},
			{
				id: "actions-review",
				type: "actions",
				position: {
					x: (columnIndex.get(REVIEW_CONTEXT_STATUS) ?? 0) * COLUMN_GAP,
					y: ACTIONS_ROW_Y,
				},
				draggable: false,
				selectable: false,
				data: { actions: reviewActions },
			},
		];

		const staticNode: Node = {
			id: "static-contexts",
			type: "static",
			position: { x: 0, y: STATIC_ROW_Y },
			draggable: false,
			selectable: false,
			data: {
				contexts: staticContexts,
				project,
				pullRequestLookup,
				onOpenKiro: (target: ContextView) => setOpenKiroContextId(target.id),
				isKiroRunning: isAgentRunning,
				hasKiroHistory: hasAgentHistory,
			} satisfies FlowBoardStaticData,
		};

		return [staticNode, ...actionNodes, ...columnNodes];
	}, [
		columnIndex,
		contextsByStatus,
		project,
		pullRequestLookup,
		isAgentRunning,
		hasAgentHistory,
		onCreateFromTasks,
		onBulkCreate,
		onShareForReview,
		onCreateFromBranches,
		onCreateFromPullRequests,
		staticContexts,
		t,
	]);

	const edges = useMemo<Edge[]>(() => {
		return CONTEXT_STATUSES.slice(0, -1).map((status, index) => {
			const target = CONTEXT_STATUSES[index + 1];
			return {
				id: `${status}->${target}`,
				source: status,
				target,
				animated: true,
				markerEnd: { type: MarkerType.ArrowClosed },
				style: { strokeWidth: 2 },
			};
		});
	}, []);

	const activeContext = activeId ? filtered.find((c) => c.id === activeId) : undefined;
	const openKiroContext = openKiroContextId
		? contexts.find((c) => c.id === openKiroContextId)
		: undefined;

	function handleDragStart(event: DragStartEvent) {
		setActiveId(String(event.active.id));
	}

	function handleDragEnd(event: DragEndEvent) {
		setActiveId(null);
		const { active, over } = event;
		if (!over) return;

		const targetStatus = String(over.id);
		if (!isContextStatus(targetStatus)) return;

		const context = contexts.find((c) => c.id === String(active.id));
		if (!context || context.isStatic || context.status === targetStatus) return;

		setContextStatus({ contextId: context.id, status: targetStatus });
	}

	return (
		<DndContext
			sensors={sensors}
			collisionDetection={closestCorners}
			onDragStart={handleDragStart}
			onDragEnd={handleDragEnd}
			onDragCancel={() => setActiveId(null)}
		>
			<div style={{ flex: 1, minHeight: 0 }}>
				<ReactFlow
					nodes={nodes}
					edges={edges}
					nodeTypes={nodeTypes}
					nodesDraggable={false}
					nodesConnectable={false}
					elementsSelectable={false}
					onNodeMouseEnter={noop}
					panOnDrag={[1, 2]}
					selectionOnDrag={false}
					zoomOnDoubleClick={false}
					fitView
					minZoom={0.2}
					maxZoom={1}
					proOptions={{ hideAttribution: true }}
				>
					<Background />
				</ReactFlow>
			</div>
			<DragOverlay>
				{activeContext ? (
					<ContextCard
						context={activeContext}
						project={project}
						pullRequestLookup={pullRequestLookup}
						dragHandle={<ContextCardDragHandle />}
					/>
				) : null}
			</DragOverlay>

			{openKiroContext && (
				<KiroConversationModal
					opened
					onClose={() => {
						keepPrompt(openKiroContext);
						setOpenKiroContextId(null);
					}}
					contextId={openKiroContext.id}
					contextName={openKiroContext.name}
					liveRun={liveRunFor(openKiroContext.id)}
					isRunning={isAgentRunning(openKiroContext.id)}
					prompt={promptFor(openKiroContext)}
					onPromptChange={(prompt) => changePrompt(openKiroContext.id, prompt)}
					onSend={(prompt) => onRunAgent({ contextId: openKiroContext.id, prompt })}
					onKill={() => onStopAgent(openKiroContext.id)}
					slashBasePath={openKiroContext.idePath}
				/>
			)}
		</DndContext>
	);
}
