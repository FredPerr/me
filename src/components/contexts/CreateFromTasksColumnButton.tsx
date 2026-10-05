import { Button, Tooltip } from "@mantine/core";
import { ListChecksIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";

type CreateFromTasksColumnButtonProps = {
	onClick: () => void;
};

export function CreateFromTasksColumnButton({ onClick }: CreateFromTasksColumnButtonProps) {
	const { t } = useTranslation();

	return (
		<Tooltip label={t("contexts.fromTasks.buttonHint")}>
			<Button
				variant="subtle"
				size="compact-xs"
				leftSection={<ListChecksIcon size={14} />}
				onClick={onClick}
			>
				{t("contexts.fromTasks.button")}
			</Button>
		</Tooltip>
	);
}
