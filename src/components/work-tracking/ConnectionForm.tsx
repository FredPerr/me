import { Alert, Button, PasswordInput, Stack, TextInput } from "@mantine/core";
import { type FormEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { DEFAULT_PROVIDER_KIND } from "@/domain/work-tracking/ProviderKind";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { WorkTrackingErrorKind } from "@/domain/work-tracking/WorkTrackingErrorKind";
import type { WorkTrackingConnections } from "@/hooks/work-tracking/useWorkTrackingConnections";
import { translateError } from "./translateError";

const BASE_URL_FIELD = "baseUrl";
const API_KEY_FIELD = "apiKey";

type ConnectionFormProps = {
	save: WorkTrackingConnections["save"];
};

export function ConnectionForm({ save }: ConnectionFormProps) {
	const { t } = useTranslation();
	const [baseUrl, setBaseUrl] = useState("");
	const [apiKey, setApiKey] = useState("");
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<WorkTrackingError | null>(null);

	async function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		setSaving(true);
		setError(null);
		try {
			await save({ kind: DEFAULT_PROVIDER_KIND, baseUrl, apiKey });
			setApiKey("");
		} catch (saveError) {
			setError(WorkTrackingError.fromUnknown(saveError));
		} finally {
			setSaving(false);
		}
	}

	const isFieldError = error?.kind === WorkTrackingErrorKind.InvalidInput;
	const errorText = error ? translateError(t, error) : null;
	const baseUrlError = isFieldError && error.field === BASE_URL_FIELD ? errorText : null;
	const apiKeyError = isFieldError && error.field === API_KEY_FIELD ? errorText : null;
	const formError = error && !baseUrlError && !apiKeyError ? errorText : null;

	return (
		<form onSubmit={handleSubmit}>
			<Stack gap="sm">
				<TextInput
					label={t("workTracking.connection.siteUrl")}
					placeholder={t("workTracking.connection.siteUrlPlaceholder")}
					description={t("workTracking.connection.siteUrlDescription")}
					value={baseUrl}
					onChange={(event) => setBaseUrl(event.currentTarget.value)}
					error={baseUrlError}
					autoComplete="url"
					required
				/>
				<PasswordInput
					label={t("workTracking.connection.apiKey")}
					description={t("workTracking.connection.apiKeyDescription")}
					value={apiKey}
					onChange={(event) => setApiKey(event.currentTarget.value)}
					error={apiKeyError}
					autoComplete="off"
					required
				/>
				{formError && (
					<Alert color="red" title={t("common.error")}>
						{formError}
					</Alert>
				)}
				<Button type="submit" loading={saving} style={{ alignSelf: "flex-start" }}>
					{t("workTracking.connection.save")}
				</Button>
			</Stack>
		</form>
	);
}
