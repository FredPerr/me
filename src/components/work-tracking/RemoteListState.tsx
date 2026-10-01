import { Alert, Button, Group, Loader, Stack, Text, VisuallyHidden } from "@mantine/core";
import { ArrowClockwiseIcon, KeyIcon } from "@phosphor-icons/react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { WorkTrackingErrorKind } from "@/domain/work-tracking/WorkTrackingErrorKind";
import { PagedListStatus } from "@/hooks/work-tracking/pagedListState";
import { translateError } from "./translateError";

type RemoteListStateProps = {
	status: PagedListStatus;
	error: WorkTrackingError | null;
	itemCount: number;
	emptyText: string;
	onRetry: () => void;
	onReplaceKey: () => void;
	children: ReactNode;
};

/**
 * Wraps a remote list column. Only the small status text is a live region, so rows appended by
 * "Load more" are not re-announced.
 */
export function RemoteListState({
	status,
	error,
	itemCount,
	emptyText,
	onRetry,
	onReplaceKey,
	children,
}: RemoteListStateProps) {
	const { t } = useTranslation();
	const isEmpty = itemCount === 0;
	const errorText = error ? translateError(t, error) : null;

	function statusText(): string {
		switch (status) {
			case PagedListStatus.Loading:
				return t("workTracking.loading");
			case PagedListStatus.Error:
				return errorText ?? "";
			case PagedListStatus.Ready:
				return isEmpty ? emptyText : t("workTracking.itemsLoaded", { count: itemCount });
			default:
				return "";
		}
	}

	const showStatusVisually = status === PagedListStatus.Ready && isEmpty;

	return (
		<Stack gap="xs">
			<Text size="sm" c="dimmed" role="status" aria-live="polite">
				{showStatusVisually ? statusText() : <VisuallyHidden>{statusText()}</VisuallyHidden>}
			</Text>
			{status === PagedListStatus.Loading && (
				<Group justify="center" py="md">
					<Loader size="sm" />
				</Group>
			)}
			{status === PagedListStatus.Error && errorText && (
				<Alert color="red" title={t("common.error")}>
					<Stack gap="xs" align="flex-start">
						<Text size="sm">{errorText}</Text>
						<Group gap="xs">
							<Button
								size="xs"
								variant="light"
								leftSection={<ArrowClockwiseIcon size={14} />}
								onClick={onRetry}
							>
								{t("workTracking.retry")}
							</Button>
							{error?.kind === WorkTrackingErrorKind.Unauthorized && (
								<Button
									size="xs"
									variant="light"
									leftSection={<KeyIcon size={14} />}
									onClick={onReplaceKey}
								>
									{t("workTracking.connection.replaceKey")}
								</Button>
							)}
						</Group>
					</Stack>
				</Alert>
			)}
			{status !== PagedListStatus.Loading && !isEmpty && children}
		</Stack>
	);
}
