import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { ActionIcon, Box, Tooltip } from "@mantine/core";
import { DotsSixVerticalIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { CreateContextParams } from "@/hooks/useContexts";
import type { Context, Project } from "@/models/Project";
import { ContextCard } from "./ContextCard";

type DraggableContextCardProps = {
	context: Context;
	project: Project;
	allContexts: Context[];
	onDelete: (contextId: string) => Promise<void> | void;
	onCreate: (params: CreateContextParams) => Promise<void>;
};

export function DraggableContextCard({
	context,
	project,
	allContexts,
	onDelete,
	onCreate,
}: DraggableContextCardProps) {
	const { t } = useTranslation();
	const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
		id: context.id,
		disabled: context.isDefault,
	});

	const style = {
		transform: CSS.Translate.toString(transform),
		opacity: isDragging ? 0.4 : 1,
		position: "relative" as const,
	};

	return (
		<Box ref={setNodeRef} style={style}>
			{!context.isDefault && (
				<Tooltip label={t("contexts.kanban.dragHandle")} openDelay={400}>
					<ActionIcon
						variant="subtle"
						color="gray"
						size="sm"
						radius={2}
						aria-label={t("contexts.kanban.dragHandle")}
						style={{
							position: "absolute",
							top: 6,
							left: 6,
							zIndex: 1,
							cursor: "grab",
							touchAction: "none",
						}}
						{...attributes}
						{...listeners}
					>
						<DotsSixVerticalIcon size={16} />
					</ActionIcon>
				</Tooltip>
			)}
			<ContextCard
				context={context}
				project={project}
				allContexts={allContexts}
				onDelete={onDelete}
				onCreate={onCreate}
			/>
		</Box>
	);
}
