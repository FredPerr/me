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
import { CONTEXT_STATUSES, type ContextStatus, isContextStatus } from "@/models/ContextStatus";
import type { Context, Project } from "@/models/Project";
import { ContextCard } from "./ContextCard";
import { DraggableContextCard } from "./DraggableContextCard";
import { KanbanColumn } from "./KanbanColumn";

type KanbanBoardProps = {
	contexts: Context[];
	project: Project;
	onDelete: (contextId: string) => void;
	onCreate: (params: CreateContextParams) => Promise<void>;
	onStatusChange: (contextId: string, status: ContextStatus) => void;
	searchFilter?: string;
};

export function KanbanBoard({
	contexts,
	project,
	onDelete,
	onCreate,
	onStatusChange,
	searchFilter,
}: KanbanBoardProps) {
	const [activeId, setActiveId] = useState<string | null>(null);
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
			<Group align="flex-start" gap="sm" wrap="nowrap" style={{ overflowX: "auto" }}>
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
					/>
				) : null}
			</DragOverlay>
		</DndContext>
	);
}
