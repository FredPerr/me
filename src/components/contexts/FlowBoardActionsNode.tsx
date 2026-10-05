import { Button, Group } from "@mantine/core";
import type { Icon } from "@phosphor-icons/react";

export type FlowBoardAction = {
	key: string;
	label: string;
	icon: Icon;
	onClick: () => void;
};

export type FlowBoardActionsData = {
	actions: FlowBoardAction[];
};

export function FlowBoardActionsNode({ data }: { data: FlowBoardActionsData }) {
	return (
		<Group gap="xs" wrap="wrap" style={{ width: 340 }}>
			{data.actions.map(({ key, label, icon: ActionIcon, onClick }) => (
				<Button
					key={key}
					variant="default"
					size="xs"
					leftSection={<ActionIcon size={16} />}
					onClick={onClick}
				>
					{label}
				</Button>
			))}
		</Group>
	);
}
