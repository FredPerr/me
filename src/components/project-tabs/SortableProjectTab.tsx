import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { ReactNode } from "react";

type SortableProjectTabProps = {
	projectTag: string;
	children: ReactNode;
};

/**
 * Makes one tab draggable within its sortable group. The drag listeners sit on
 * a wrapper around the whole tab; the PointerSensor's activation distance means
 * a plain click still reaches the tab (selecting it) and a right-click still
 * opens the context menu — only a deliberate drag starts a reorder.
 */
export function SortableProjectTab({ projectTag, children }: SortableProjectTabProps) {
	const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
		id: projectTag,
	});

	const style = {
		display: "inline-flex",
		transform: CSS.Translate.toString(transform),
		transition,
		opacity: isDragging ? 0.4 : 1,
		touchAction: "none" as const,
	};

	return (
		<div ref={setNodeRef} style={style} {...attributes} {...listeners}>
			{children}
		</div>
	);
}
