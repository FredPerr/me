import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { Box } from "@mantine/core";
import type { PullRequestLookup } from "@/hooks/useContextPullRequestStatus";
import type { CreateContextParams } from "@/hooks/useContexts";
import type { Context, Project } from "@/models/Project";
import { ContextCard } from "./ContextCard";
import { ContextCardDragHandle } from "./ContextCardDragHandle";

type DraggableContextCardProps = {
	context: Context;
	project: Project;
	allContexts: Context[];
	onDelete: (contextId: string) => Promise<void> | void;
	onCreate: (params: CreateContextParams) => Promise<void>;
	pullRequestLookup?: PullRequestLookup;
	onExpandedChange?: (contextId: string, expanded: boolean) => void;
	onStaticChange?: (contextId: string, isStatic: boolean) => void;
	onOpenKiro?: (context: Context) => void;
	isKiroRunning?: boolean;
	hasKiroHistory?: boolean;
};

export function DraggableContextCard({
	context,
	project,
	allContexts,
	onDelete,
	onCreate,
	pullRequestLookup,
	onExpandedChange,
	onStaticChange,
	onOpenKiro,
	isKiroRunning,
	hasKiroHistory,
}: DraggableContextCardProps) {
	const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
		id: context.id,
		disabled: context.isDefault,
	});

	const style = {
		transform: CSS.Translate.toString(transform),
		opacity: isDragging ? 0.4 : 1,
	};

	const dragHandle = context.isDefault ? undefined : (
		<ContextCardDragHandle attributes={attributes} listeners={listeners} />
	);

	return (
		<Box ref={setNodeRef} style={style}>
			<ContextCard
				context={context}
				project={project}
				allContexts={allContexts}
				onDelete={onDelete}
				onCreate={onCreate}
				pullRequestLookup={pullRequestLookup}
				onExpandedChange={onExpandedChange}
				onStaticChange={onStaticChange}
				onOpenKiro={onOpenKiro}
				isKiroRunning={isKiroRunning}
				hasKiroHistory={hasKiroHistory}
				dragHandle={dragHandle}
			/>
		</Box>
	);
}
