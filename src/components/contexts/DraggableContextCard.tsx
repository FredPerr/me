import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { Box } from "@mantine/core";
import type { ContextView, ProjectView } from "@/bindings";
import type { PullRequestLookup } from "@/hooks/useContextPullRequestStatus";
import { ContextCard } from "./ContextCard";
import { ContextCardDragHandle } from "./ContextCardDragHandle";

type DraggableContextCardProps = {
	context: ContextView;
	project: ProjectView;
	pullRequestLookup?: PullRequestLookup;
	onOpenKiro?: (context: ContextView) => void;
	isKiroRunning?: boolean;
	hasKiroHistory?: boolean;
};

export function DraggableContextCard({
	context,
	project,
	pullRequestLookup,
	onOpenKiro,
	isKiroRunning,
	hasKiroHistory,
}: DraggableContextCardProps) {
	const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
		id: context.id,
		disabled: context.isStatic,
	});

	const style = {
		transform: CSS.Translate.toString(transform),
		opacity: isDragging ? 0.4 : 1,
	};

	const dragHandle = context.isStatic ? undefined : (
		<ContextCardDragHandle attributes={attributes} listeners={listeners} />
	);

	return (
		<Box ref={setNodeRef} style={style}>
			<ContextCard
				context={context}
				project={project}
				pullRequestLookup={pullRequestLookup}
				onOpenKiro={onOpenKiro}
				isKiroRunning={isKiroRunning}
				hasKiroHistory={hasKiroHistory}
				dragHandle={dragHandle}
			/>
		</Box>
	);
}
