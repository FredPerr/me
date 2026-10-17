import type {
	WorkTrackingCommandError,
	WorkTrackingErrorKind,
	WorkTrackingInvalidReason,
} from "@/bindings";

export const ERROR_KEYS: Record<WorkTrackingErrorKind, string> = {
	notConfigured: "workTracking.errors.notConfigured",
	invalidInput: "workTracking.errors.invalidInput.generic",
	unauthorized: "workTracking.errors.unauthorized",
	forbidden: "workTracking.errors.forbidden",
	notFound: "workTracking.errors.notFound",
	rateLimited: "workTracking.errors.rateLimited",
	timeout: "workTracking.errors.timeout",
	network: "workTracking.errors.network",
	providerError: "workTracking.errors.providerError",
	invalidResponse: "workTracking.errors.invalidResponse",
	storageError: "workTracking.errors.storageError",
};

export const REASON_KEYS: Record<WorkTrackingInvalidReason, string> = {
	required: "workTracking.errors.invalidInput.required",
	tooLong: "workTracking.errors.invalidInput.tooLong",
	invalidFormat: "workTracking.errors.invalidInput.invalidFormat",
	httpsRequired: "workTracking.errors.invalidInput.httpsRequired",
	unsupportedComponent: "workTracking.errors.invalidInput.unsupportedComponent",
	outOfRange: "workTracking.errors.invalidInput.outOfRange",
	unsupported: "workTracking.errors.invalidInput.unsupported",
};

export const RATE_LIMITED_RETRY_AFTER_KEY = "workTracking.errors.rateLimitedRetryAfter";

type ErrorMessage = {
	key: string;
	values?: Record<string, unknown>;
};

export function errorMessageKey(error: WorkTrackingCommandError): ErrorMessage {
	switch (error.kind) {
		case "invalidInput":
			return error.reason !== null
				? { key: REASON_KEYS[error.reason] }
				: { key: ERROR_KEYS.invalidInput };
		case "rateLimited":
			return error.retryAfterSeconds !== null
				? { key: RATE_LIMITED_RETRY_AFTER_KEY, values: { seconds: error.retryAfterSeconds } }
				: { key: ERROR_KEYS.rateLimited };
		default:
			return { key: ERROR_KEYS[error.kind] };
	}
}
