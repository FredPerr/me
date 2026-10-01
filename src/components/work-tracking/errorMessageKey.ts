import { InvalidInputReason } from "@/domain/work-tracking/InvalidInputReason";
import type { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { WorkTrackingErrorKind } from "@/domain/work-tracking/WorkTrackingErrorKind";

export const ERROR_KEYS: Record<WorkTrackingErrorKind, string> = {
	[WorkTrackingErrorKind.NotConfigured]: "workTracking.errors.notConfigured",
	[WorkTrackingErrorKind.InvalidInput]: "workTracking.errors.invalidInput.generic",
	[WorkTrackingErrorKind.Unauthorized]: "workTracking.errors.unauthorized",
	[WorkTrackingErrorKind.Forbidden]: "workTracking.errors.forbidden",
	[WorkTrackingErrorKind.NotFound]: "workTracking.errors.notFound",
	[WorkTrackingErrorKind.RateLimited]: "workTracking.errors.rateLimited",
	[WorkTrackingErrorKind.Timeout]: "workTracking.errors.timeout",
	[WorkTrackingErrorKind.Network]: "workTracking.errors.network",
	[WorkTrackingErrorKind.ProviderError]: "workTracking.errors.providerError",
	[WorkTrackingErrorKind.InvalidResponse]: "workTracking.errors.invalidResponse",
	[WorkTrackingErrorKind.StorageError]: "workTracking.errors.storageError",
};

export const REASON_KEYS: Record<InvalidInputReason, string> = {
	[InvalidInputReason.Required]: "workTracking.errors.invalidInput.required",
	[InvalidInputReason.TooLong]: "workTracking.errors.invalidInput.tooLong",
	[InvalidInputReason.InvalidFormat]: "workTracking.errors.invalidInput.invalidFormat",
	[InvalidInputReason.HttpsRequired]: "workTracking.errors.invalidInput.httpsRequired",
	[InvalidInputReason.UnsupportedComponent]:
		"workTracking.errors.invalidInput.unsupportedComponent",
	[InvalidInputReason.OutOfRange]: "workTracking.errors.invalidInput.outOfRange",
	[InvalidInputReason.Unsupported]: "workTracking.errors.invalidInput.unsupported",
};

export const RATE_LIMITED_RETRY_AFTER_KEY = "workTracking.errors.rateLimitedRetryAfter";

type ErrorMessage = {
	key: string;
	values?: Record<string, unknown>;
};

const KNOWN_REASONS: ReadonlySet<string> = new Set(Object.values(InvalidInputReason));

function isInvalidInputReason(reason: string | undefined): reason is InvalidInputReason {
	return reason !== undefined && KNOWN_REASONS.has(reason);
}

export function errorMessageKey(error: WorkTrackingError): ErrorMessage {
	switch (error.kind) {
		case WorkTrackingErrorKind.InvalidInput:
			return isInvalidInputReason(error.reason)
				? { key: REASON_KEYS[error.reason] }
				: { key: ERROR_KEYS[WorkTrackingErrorKind.InvalidInput] };
		case WorkTrackingErrorKind.RateLimited:
			return error.retryAfterSeconds !== undefined
				? { key: RATE_LIMITED_RETRY_AFTER_KEY, values: { seconds: error.retryAfterSeconds } }
				: { key: ERROR_KEYS[WorkTrackingErrorKind.RateLimited] };
		default:
			return { key: ERROR_KEYS[error.kind] };
	}
}
