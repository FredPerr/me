import { Alert, Button, Group, Modal, Stack, Text } from "@mantine/core";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { translateError } from "./translateError";

type RemoveConnectionModalProps = {
	opened: boolean;
	onClose: () => void;
	connection: ProviderConnection;
	onRemove: (connection: ProviderConnection) => Promise<void>;
};

export function RemoveConnectionModal({
	opened,
	onClose,
	connection,
	onRemove,
}: RemoveConnectionModalProps) {
	const { t } = useTranslation();
	const [removing, setRemoving] = useState(false);
	const [error, setError] = useState<WorkTrackingError | null>(null);

	function handleClose() {
		setError(null);
		onClose();
	}

	async function handleRemove() {
		setRemoving(true);
		setError(null);
		try {
			await onRemove(connection);
			onClose();
		} catch (removeError) {
			setError(WorkTrackingError.fromUnknown(removeError));
		} finally {
			setRemoving(false);
		}
	}

	return (
		<Modal
			opened={opened}
			onClose={handleClose}
			centered
			title={t("workTracking.connection.removeTitle")}
		>
			<Stack gap="sm">
				<Text size="sm">
					{t("workTracking.connection.removeConfirm", { site: connection.baseUrl })}
				</Text>
				{error && (
					<Alert color="red" title={t("common.error")}>
						{translateError(t, error)}
					</Alert>
				)}
				<Group justify="flex-end" gap="sm">
					<Button variant="default" onClick={handleClose} disabled={removing}>
						{t("common.cancel")}
					</Button>
					<Button color="red" onClick={handleRemove} loading={removing}>
						{t("workTracking.connection.remove")}
					</Button>
				</Group>
			</Stack>
		</Modal>
	);
}
