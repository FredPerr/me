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
import { Group } from "@mantine/core";
import { useMemo, useState } from "react";
import type { CreateContextParams } from "@/hooks/useContexts";
import type { KiroConversation } from "@/models/ai-session/KiroConversation";
import { CONTEXT_STATUSES, type ContextStatus, isContextStatus } from "@/models/ContextStatus";
import type { Context, Project } from "@/models/Project";
import { ContextCard } from "./ContextCard";
import { ContextCardDragHandle } from "./ContextCardDragHandle";
import { DraggableContextCard } from "./DraggableContextCard";
import { KanbanColumn } from "./KanbanColumn";
import { KiroConversationModal } from "./KiroConversationModal";

type KanbanBoardProps = {
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
};

export function KanbanBoard({
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
}: KanbanBoardProps) {
	const [activeId, setActiveId] = useState<string | null>(null);
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
			groups.get(context.status)?.push(context);
		}
		return groups;
	}, [filtered]);

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
			<Group
				align="stretch"
				gap={4}
				wrap="nowrap"
				style={{ flex: 1, minHeight: 0, overflowX: "auto" }}
			>
				{CONTEXT_STATUSES.map((status) => {
					const columnContexts = contextsByStatus.get(status) ?? [];
					return (
						<KanbanColumn key={status} status={status} count={columnContexts.length}>
							{columnContexts.map((context) => (
								<DraggableContextCard
									key={context.id}
									context={context}
									project={project}
									allContexts={contexts}
									onDelete={onDelete}
									onCreate={onCreate}
									onExpandedChange={onExpandedChange}
									onOpenKiro={(target) => setOpenKiroContextId(target.id)}
									isKiroRunning={isKiroRunning(context.id)}
									hasKiroHistory={(getKiroConversation(context.id)?.turns.length ?? 0) > 0}
								/>
							))}
						</KanbanColumn>
					);
				})}
			</Group>
			<DragOverlay>
				{activeContext ? (
					<ContextCard
						context={activeContext}
						project={project}
						allContexts={contexts}
						onDelete={onDelete}
						onCreate={onCreate}
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
