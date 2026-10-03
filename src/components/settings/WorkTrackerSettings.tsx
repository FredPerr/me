import { Stack, Text, Title } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ConnectionPanel } from "@/components/work-tracking/ConnectionPanel";
import { ReplaceKeyModal } from "@/components/work-tracking/ReplaceKeyModal";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { DEFAULT_PROVIDER_KIND } from "@/domain/work-tracking/ProviderKind";
import { useWorkTrackingConnections } from "@/hooks/work-tracking/useWorkTrackingConnections";

export function WorkTrackerSettings() {
	const { t } = useTranslation();
	const { connections, status, error, save, remove, reload } = useWorkTrackingConnections();
	const [replaceKeyOpened, setReplaceKeyOpened] = useState(false);
	const connection =
		connections.find((candidate) => candidate.kind === DEFAULT_PROVIDER_KIND) ?? null;

	async function handleRemove(removedConnection: ProviderConnection) {
		await remove(removedConnection.id);
		setReplaceKeyOpened(false);
	}

	return (
		<Stack gap="sm">
			<Title order={3}>{t("workTracking.connection.title")}</Title>
			<Text size="sm" c="dimmed">
				{t("workTracking.connection.description")}
			</Text>
			<ConnectionPanel
				connection={connection}
				status={status}
				error={error}
				save={save}
				remove={handleRemove}
				onReplaceKey={() => setReplaceKeyOpened(true)}
				onRetry={reload}
			/>
			<ReplaceKeyModal
				opened={replaceKeyOpened}
				onClose={() => setReplaceKeyOpened(false)}
				connection={connection}
				save={save}
			/>
		</Stack>
	);
}
