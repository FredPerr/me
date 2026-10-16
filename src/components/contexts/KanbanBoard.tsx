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
import { Group, ScrollArea } from "@mantine/core";
import { useMemo, useState } from "react";
import type { ContextView, ProjectView } from "@/bindings";
import { useContextPullRequests } from "@/hooks/gitHosting/useContextPullRequests";
import { useKiroPromptDrafts } from "@/hooks/useKiroPromptDrafts";
import { useSetContextStatus } from "@/hooks/workspace/useContextMutations";
import type { KiroConversation } from "@/models/ai-session/KiroConversation";
import {
	CONTEXT_STATUSES,
	type ContextStatus,
	IDLE_CONTEXT_STATUS,
	isContextStatus,
} from "@/models/ContextStatus";
import { ContextCard } from "./ContextCard";
import { ContextCardDragHandle } from "./ContextCardDragHandle";
import { CreateFromTasksColumnButton } from "./CreateFromTasksColumnButton";
import { DraggableContextCard } from "./DraggableContextCard";
import { KanbanColumn } from "./KanbanColumn";
import { KiroConversationModal } from "./KiroConversationModal";

type KanbanBoardProps = {
	project: ProjectView;
	onRunKiro: (context: ContextView, prompt: string) => Promise<void>;
	onKillKiro: (contextId: string) => Promise<void>;
	isKiroRunning: (contextId: string) => boolean;
	getKiroConversation: (contextId: string) => KiroConversation | undefined;
	searchFilter?: string;
	onCreateFromTasks?: () => void;
};

export function KanbanBoard({
	project,
	onRunKiro,
	onKillKiro,
	isKiroRunning,
	getKiroConversation,
	searchFilter,
	onCreateFromTasks,
}: KanbanBoardProps) {
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
			<ScrollArea
				type="always"
				scrollbarSize={10}
				scrollbars="x"
				style={{ flex: 1, minHeight: 0 }}
				styles={{
					viewport: { height: "100%" },
					content: { height: "100%", display: "flex" },
				}}
			>
				<Group align="stretch" gap={4} wrap="nowrap" style={{ height: "100%", minHeight: 0 }}>
					{CONTEXT_STATUSES.map((status) => {
						const columnContexts = contextsByStatus.get(status) ?? [];
						return (
							<KanbanColumn
								key={status}
								status={status}
								count={columnContexts.length}
								headerAction={
									status === IDLE_CONTEXT_STATUS && onCreateFromTasks ? (
										<CreateFromTasksColumnButton onClick={onCreateFromTasks} />
									) : undefined
								}
							>
								{columnContexts.map((context) => (
									<DraggableContextCard
										key={context.id}
										context={context}
										project={project}
										pullRequestLookup={pullRequestLookup}
										onOpenKiro={(target) => setOpenKiroContextId(target.id)}
										isKiroRunning={isKiroRunning(context.id)}
										hasKiroHistory={(getKiroConversation(context.id)?.turns.length ?? 0) > 0}
									/>
								))}
							</KanbanColumn>
						);
					})}
				</Group>
			</ScrollArea>
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
					conversation={getKiroConversation(openKiroContext.id)}
					isRunning={isKiroRunning(openKiroContext.id)}
					prompt={promptFor(openKiroContext)}
					onPromptChange={(prompt) => changePrompt(openKiroContext.id, prompt)}
					onSend={(prompt) => onRunKiro(openKiroContext, prompt)}
					onKill={() => onKillKiro(openKiroContext.id)}
					slashBasePath={openKiroContext.idePath}
				/>
			)}
		</DndContext>
	);
}
