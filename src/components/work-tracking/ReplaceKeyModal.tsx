import { Alert, Button, Group, Modal, PasswordInput, Stack, TextInput } from "@mantine/core";
import { type FormEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProviderConnection } from "@/domain/work-tracking/ProviderConnection";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { WorkTrackingErrorKind } from "@/domain/work-tracking/WorkTrackingErrorKind";
import type { WorkTrackingConnections } from "@/hooks/work-tracking/useWorkTrackingConnections";
import { translateError } from "./translateError";

const API_KEY_FIELD = "apiKey";

type ReplaceKeyModalProps = {
	opened: boolean;
	onClose: () => void;
	connection: ProviderConnection | null;
	save: WorkTrackingConnections["save"];
};

export function ReplaceKeyModal({ opened, onClose, connection, save }: ReplaceKeyModalProps) {
	const { t } = useTranslation();
	const [apiKey, setApiKey] = useState("");
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<WorkTrackingError | null>(null);

	function handleClose() {
		setApiKey("");
		setError(null);
		onClose();
	}

	async function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!connection) return;
		setSaving(true);
		setError(null);
		try {
			await save({
				kind: connection.kind,
				baseUrl: connection.baseUrl,
				displayName: connection.displayName,
				apiKey,
			});
			setApiKey("");
			onClose();
		} catch (saveError) {
			setError(WorkTrackingError.fromUnknown(saveError));
		} finally {
			setSaving(false);
		}
	}

	const errorText = error ? translateError(t, error) : null;
	const isApiKeyError =
		error?.kind === WorkTrackingErrorKind.InvalidInput && error.field === API_KEY_FIELD;

	return (
		<Modal
			opened={opened && connection !== null}
			onClose={handleClose}
			centered
			title={t("workTracking.connection.replaceKeyTitle")}
		>
			<form onSubmit={handleSubmit}>
				<Stack gap="sm">
					<TextInput
						label={t("workTracking.connection.siteUrl")}
						value={connection?.baseUrl ?? ""}
						readOnly
					/>
					<PasswordInput
						label={t("workTracking.connection.apiKey")}
						description={t("workTracking.connection.apiKeyDescription")}
						value={apiKey}
						onChange={(event) => setApiKey(event.currentTarget.value)}
						error={isApiKeyError ? errorText : null}
						autoComplete="off"
						data-autofocus
						required
					/>
					{errorText && !isApiKeyError && (
						<Alert color="red" title={t("common.error")}>
							{errorText}
						</Alert>
					)}
					<Group justify="flex-end" gap="sm">
						<Button variant="default" onClick={handleClose} disabled={saving}>
							{t("common.cancel")}
						</Button>
						<Button type="submit" loading={saving}>
							{t("common.save")}
						</Button>
					</Group>
				</Stack>
			</form>
		</Modal>
	);
}
