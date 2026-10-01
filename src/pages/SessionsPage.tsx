import { Alert, Card, Container, Loader, Stack, Text, Title } from "@mantine/core";
import { RobotIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { SessionCard } from "@/components/ai-sessions/SessionCard";
import { SpawnSessionForm } from "@/components/ai-sessions/SpawnSessionForm";
import { useAiSessions } from "@/hooks/useAiSessions";

export function SessionsPage() {
	const { t } = useTranslation();
	const { sessions, adapters, loadingAdapters, spawn, kill, clear } = useAiSessions();

	return (
		<Container size="md" py="lg">
			<Stack gap="lg">
				<Title order={2}>{t("aiSessions.title")}</Title>

				{loadingAdapters ? (
					<Loader size="sm" />
				) : adapters.length === 0 ? (
					<Alert color="yellow" title={t("aiSessions.noAdaptersTitle")}>
						{t("aiSessions.noAdaptersDescription")}
					</Alert>
				) : (
					<Card withBorder padding="md">
						<SpawnSessionForm adapters={adapters} onSpawn={spawn} />
					</Card>
				)}

				<Stack gap="xs">
					<Title order={4}>{t("aiSessions.activeSessions")}</Title>
					{sessions.length === 0 ? (
						<Stack align="center" gap="sm" py="xl">
							<RobotIcon size={40} color="var(--mantine-color-dark-2)" />
							<Text size="sm" c="dimmed">
								{t("aiSessions.noSessions")}
							</Text>
						</Stack>
					) : (
						sessions.map((session) => (
							<SessionCard
								key={session.sessionId}
								session={session}
								onKill={kill}
								onClear={clear}
							/>
						))
					)}
				</Stack>
			</Stack>
		</Container>
	);
}
