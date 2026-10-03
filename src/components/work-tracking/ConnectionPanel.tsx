import {
	Alert,
	Badge,
	Button,
	Group,
	Loader,
	Paper,
	Stack,
	Text,
	VisuallyHidden,
} from "@mantine/core";
import { KeyIcon, TrashIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import type { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import {
	ConnectionsStatus,
	type WorkTrackingConnections,
} from "@/hooks/work-tracking/useWorkTrackingConnections";
import { ConnectionForm } from "./ConnectionForm";
import { RemoveConnectionModal } from "./RemoveConnectionModal";
import { translateError } from "./translateError";

type ConnectionPanelProps = {
	connection: ProviderConnection | null;
	status: ConnectionsStatus;
	error: WorkTrackingError | null;
	save: WorkTrackingConnections["save"];
	remove: (connection: ProviderConnection) => Promise<void>;
	onReplaceKey: () => void;
	onRetry: () => void;
};

export function ConnectionPanel({
	connection,
	status,
	error,
	save,
	remove,
	onReplaceKey,
	onRetry,
}: ConnectionPanelProps) {
	const { t } = useTranslation();

	return (
		<Paper withBorder p="md" radius="md">
			<Stack gap="sm">
				{status === ConnectionsStatus.Loading && (
					<Group>
						<Loader size="sm" />
						<VisuallyHidden>{t("workTracking.loading")}</VisuallyHidden>
					</Group>
				)}
				{status === ConnectionsStatus.Error && error && (
					<Alert color="red" title={t("common.error")}>
						<Stack gap="xs" align="flex-start">
							<Text size="sm">{translateError(t, error)}</Text>
							<Button size="xs" variant="light" onClick={onRetry}>
								{t("workTracking.retry")}
							</Button>
						</Stack>
					</Alert>
				)}
				{status === ConnectionsStatus.Ready &&
					(connection ? (
						<ConnectedSummary connection={connection} remove={remove} onReplaceKey={onReplaceKey} />
					) : (
						<ConnectionForm save={save} />
					))}
			</Stack>
		</Paper>
	);
}

type ConnectedSummaryProps = {
	connection: ProviderConnection;
	remove: (connection: ProviderConnection) => Promise<void>;
	onReplaceKey: () => void;
};

function ConnectedSummary({ connection, remove, onReplaceKey }: ConnectedSummaryProps) {
	const { t } = useTranslation();
	const [removeOpened, setRemoveOpened] = useState(false);

	return (
		<Group justify="space-between" wrap="wrap" gap="sm">
			<Stack gap={4}>
				<Group gap="xs">
					<Badge variant="light">{t(`workTracking.providers.${connection.kind}`)}</Badge>
					<Badge variant="light" color="green">
						{t("workTracking.connection.connected")}
					</Badge>
					{!connection.credentialConfigured && (
						<Badge variant="light" color="yellow">
							{t("workTracking.connection.credentialMissing")}
						</Badge>
					)}
				</Group>
				<Text fw={500}>{connection.displayName}</Text>
				<Text size="sm" c="dimmed">
					{connection.baseUrl}
				</Text>
			</Stack>
			<Group gap="sm">
				<Button variant="default" leftSection={<KeyIcon size={16} />} onClick={onReplaceKey}>
					{t("workTracking.connection.replaceKey")}
				</Button>
				<Button
					variant="light"
					color="red"
					leftSection={<TrashIcon size={16} />}
					onClick={() => setRemoveOpened(true)}
				>
					{t("workTracking.connection.remove")}
				</Button>
			</Group>
			<RemoveConnectionModal
				opened={removeOpened}
				onClose={() => setRemoveOpened(false)}
				connection={connection}
				onRemove={remove}
			/>
		</Group>
	);
}
