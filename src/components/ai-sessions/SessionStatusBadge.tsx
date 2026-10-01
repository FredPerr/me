import { Badge } from "@mantine/core";
import { useTranslation } from "react-i18next";
import type { SessionStatus } from "@/models/ai-session/AiSession";

const STATUS_COLORS: Record<SessionStatus, string> = {
	running: "blue",
	completed: "green",
	failed: "red",
	killed: "gray",
};

export function SessionStatusBadge({ status }: { status: SessionStatus }) {
	const { t } = useTranslation();

	return (
		<Badge color={STATUS_COLORS[status]} variant="light" size="sm">
			{t(`aiSessions.status.${status}`)}
		</Badge>
	);
}
