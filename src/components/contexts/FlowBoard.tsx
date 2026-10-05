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
import { useContextPullRequestStatus } from "@/hooks/useContextPullRequestStatus";
import type { CreateContextParams } from "@/hooks/useContexts";
import type { KiroConversation } from "@/models/ai-session/KiroConversation";
import {
	CONTEXT_STATUSES,
	type ContextStatus,
	IDLE_CONTEXT_STATUS,
	isContextStatus,
} from "@/models/ContextStatus";
import type { Context, Project } from "@/models/Project";
import { ContextCard } from "./ContextCard";
import { ContextCardDragHandle } from "./ContextCardDragHandle";
import { type FlowBoardAction, FlowBoardActionsNode } from "./FlowBoardActionsNode";
import { type FlowBoardColumnData, FlowBoardColumnNode } from "./FlowBoardColumnNode";
import { type FlowBoardStaticData, FlowBoardStaticNode } from "./FlowBoardStaticNode";
import { KiroConversationModal } from "./KiroConversationModal";

type FlowBoardProps = {
	contexts: Context[];
	project: Project;
	onDelete: (contextId: string) => void;
	onCreate: (params: CreateContextParams) => Promise<void>;
	onStatusChange: (contextId: string, status: ContextStatus) => void;
	onExpandedChange: (contextId: string, expanded: boolean) => void;
	onRunKiro: (context: Context, prompt: string) => Promise<void>;
	onKillKiro: (contextId: string) => Promise<void>;
	isKiroRunning: (contextId: string) => boolean;
	getKiroConversation: (contextId: string) => KiroConversation | undefined;
	searchFilter?: string;
	onCreateFromTasks?: () => void;
	onBulkCreate: () => void;
	onShareForReview: () => void;
	onCreateFromBranches: () => void;
	onCreateFromPullRequests: () => void;
	onStaticChange: (contextId: string, isStatic: boolean) => void;
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
	contexts,
	project,
	onDelete,
	onCreate,
	onStatusChange,
	onExpandedChange,
	onRunKiro,
	onKillKiro,
	isKiroRunning,
	getKiroConversation,
	searchFilter,
	onCreateFromTasks,
	onBulkCreate,
	onShareForReview,
	onCreateFromBranches,
	onCreateFromPullRequests,
	onStaticChange,
}: FlowBoardProps) {
	const { t } = useTranslation();
	const [activeId, setActiveId] = useState<string | null>(null);
	const { lookup: pullRequestLookup } = useContextPullRequestStatus(project);
	const [openKiroContextId, setOpenKiroContextId] = useState<string | null>(null);
	const [kiroPromptDrafts, setKiroPromptDrafts] = useState<Record<string, string>>({});
	const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }));

	const filtered = useMemo(() => {
		const query = searchFilter?.toLowerCase().trim();
		if (!query) return contexts;
		return contexts.filter((c) => c.name.toLowerCase().includes(query));
	}, [contexts, searchFilter]);

	const contextsByStatus = useMemo(() => {
		const groups = new Map<ContextStatus, Context[]>(
			CONTEXT_STATUSES.map((status) => [status, []]),
		);
		for (const context of filtered) {
			if (context.isEffectivelyStatic) continue;
			groups.get(context.status)?.push(context);
		}
		return groups;
	}, [filtered]);

	const staticContexts = useMemo(
		() => filtered.filter((context) => context.isEffectivelyStatic),
		[filtered],
	);

	const getKiroHistoryCount = useCallback(
		(contextId: string) => getKiroConversation(contextId)?.turns.length ?? 0,
		[getKiroConversation],
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
				allContexts: contexts,
				pullRequestLookup,
				onDelete,
				onCreate,
				onExpandedChange,
				onStaticChange,
				onOpenKiro: (target: Context) => setOpenKiroContextId(target.id),
				isKiroRunning,
				getKiroHistoryCount,
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
				allContexts: contexts,
				pullRequestLookup,
				onDelete,
				onCreate,
				onExpandedChange,
				onStaticChange,
				onOpenKiro: (target: Context) => setOpenKiroContextId(target.id),
				isKiroRunning,
				getKiroHistoryCount,
			} satisfies FlowBoardStaticData,
		};

		return [staticNode, ...actionNodes, ...columnNodes];
	}, [
		columnIndex,
		contextsByStatus,
		contexts,
		project,
		pullRequestLookup,
		onDelete,
		onCreate,
		onExpandedChange,
		onStaticChange,
		isKiroRunning,
		getKiroHistoryCount,
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
		if (!context || context.isDefault || context.status === targetStatus) return;

		onStatusChange(context.id, targetStatus);
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
						allContexts={contexts}
						onDelete={onDelete}
						onCreate={onCreate}
						pullRequestLookup={pullRequestLookup}
						dragHandle={<ContextCardDragHandle />}
					/>
				) : null}
			</DragOverlay>

			{openKiroContext && (
				<KiroConversationModal
					opened
					onClose={() => setOpenKiroContextId(null)}
					contextId={openKiroContext.id}
					contextName={openKiroContext.name}
					conversation={getKiroConversation(openKiroContext.id)}
					isRunning={isKiroRunning(openKiroContext.id)}
					prompt={kiroPromptDrafts[openKiroContext.id] ?? openKiroContext.preprompt ?? ""}
					onPromptChange={(prompt) =>
						setKiroPromptDrafts((current) => ({ ...current, [openKiroContext.id]: prompt }))
					}
					onSend={(prompt) => onRunKiro(openKiroContext, prompt)}
					onKill={() => onKillKiro(openKiroContext.id)}
					slashBasePath={project.resolveContextIdePathCandidates(openKiroContext)[0]}
				/>
			)}
		</DndContext>
	);
}
