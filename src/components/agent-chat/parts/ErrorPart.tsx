import { Alert } from "@mantine/core";
import { WarningIcon } from "@phosphor-icons/react";
import type { ErrorPart as ErrorPartModel } from "@/models/agent-chat/AgentMessage";

type ErrorPartProps = {
	part: ErrorPartModel;
};

/** Renders an error surfaced within an agent message. */
export function ErrorPart({ part }: ErrorPartProps) {
	return (
		<Alert variant="light" color="red" icon={<WarningIcon size={18} />}>
			{part.message}
		</Alert>
	);
}
