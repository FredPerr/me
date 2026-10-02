import { UnstyledButton } from "@mantine/core";
import { DotsSixVerticalIcon } from "@phosphor-icons/react";
import type { DOMAttributes, HTMLAttributes } from "react";
import { useTranslation } from "react-i18next";

type ContextCardDragHandleProps = {
	attributes?: HTMLAttributes<HTMLElement>;
	listeners?: DOMAttributes<HTMLElement>;
};

export function ContextCardDragHandle({ attributes, listeners }: ContextCardDragHandleProps) {
	const { t } = useTranslation();

	return (
		<UnstyledButton
			aria-label={t("contexts.kanban.dragHandle")}
			title={t("contexts.kanban.dragHandle")}
			style={{
				display: "flex",
				alignItems: "center",
				justifyContent: "center",
				cursor: "grab",
				touchAction: "none",
				color: "var(--mantine-color-dark-2)",
				backgroundColor: "transparent",
			}}
			{...attributes}
			{...listeners}
		>
			<DotsSixVerticalIcon size={16} />
		</UnstyledButton>
	);
}
