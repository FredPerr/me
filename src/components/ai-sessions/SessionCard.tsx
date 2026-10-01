import { ActionIcon, Button, Card, Group, Stack, Text, Tooltip } from "@mantine/core";
import { StopCircleIcon, TrashIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { SessionLogViewer } from "@/components/ai-sessions/SessionLogViewer";
import { SessionStatusBadge } from "@/components/ai-sessions/SessionStatusBadge";
import { type AiSession, isSessionActive } from "@/models/ai-session/AiSession";

type SessionCardProps = {
	session: AiSession;
	onKill: (sessionId: string) => Promise<void>;
	onClear: (sessionId: string) => void;
};

export function SessionCard({ session, onKill, onClear }: SessionCardProps) {
	const { t } = useTranslation();
	const [killing, setKilling] = useState(false);
	const active = isSessionActive(session);

	async function handleKill() {
		setKilling(true);
		try {
			await onKill(session.sessionId);
		} finally {
			setKilling(false);
		}
	}

	return (
		<Card withBorder padding="sm">
			<Stack gap="xs">
				<Group justify="space-between" gap="xs" wrap="nowrap" align="flex-start">
					<Stack gap={2} style={{ minWidth: 0 }}>
						<Group gap="xs" wrap="nowrap">
							<Text fw={600} size="sm" truncate>
								{session.adapterName}
							</Text>
							<SessionStatusBadge status={session.status} />
							{session.pid !== null && (
								<Text size="xs" c="dimmed">
									{t("aiSessions.pidLabel", { pid: session.pid })}
								</Text>
							)}
						</Group>
						<Text size="xs" c="dimmed" truncate>
							{session.workingDirectory}
						</Text>
					</Stack>

					<Group gap="xs" wrap="nowrap">
						{active ? (
							<Button
								size="xs"
								color="red"
								variant="light"
								leftSection={<StopCircleIcon size={16} />}
								loading={killing}
								onClick={handleKill}
							>
								{t("aiSessions.killSession")}
							</Button>
						) : (
							<Tooltip label={t("aiSessions.clearSession")} position="left">
								<ActionIcon
									variant="subtle"
									color="gray"
									onClick={() => onClear(session.sessionId)}
									aria-label={t("aiSessions.clearSession")}
								>
									<TrashIcon size={16} />
								</ActionIcon>
							</Tooltip>
						)}
					</Group>
				</Group>

				<Text size="xs" c="dimmed" lineClamp={2}>
					{session.prompt}
				</Text>

				{session.error && (
					<Text size="xs" c="red.4">
						{session.error}
					</Text>
				)}

				<SessionLogViewer logs={session.logs} follow={active} />
			</Stack>
		</Card>
	);
}
