import { WorkTrackingErrorKind } from "./WorkTrackingErrorKind";

type WorkTrackingErrorDetails = {
	field?: string;
	reason?: string;
	retryAfterSeconds?: number;
	status?: number;
};

const KNOWN_KINDS: ReadonlySet<string> = new Set(Object.values(WorkTrackingErrorKind));

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null;
}

function isWorkTrackingErrorKind(value: unknown): value is WorkTrackingErrorKind {
	return typeof value === "string" && KNOWN_KINDS.has(value);
}

function stringOrUndefined(value: unknown): string | undefined {
	return typeof value === "string" ? value : undefined;
}

function finiteNumberOrUndefined(value: unknown): number | undefined {
	return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

export class WorkTrackingError extends Error {
	readonly kind: WorkTrackingErrorKind;
	// `declare` keeps undefined details as absent keys instead of own properties set to undefined.
	declare readonly field?: string;
	declare readonly reason?: string;
	declare readonly retryAfterSeconds?: number;
	declare readonly status?: number;

	constructor(
		kind: WorkTrackingErrorKind,
		message: string,
		details: WorkTrackingErrorDetails = {},
	) {
		super(message);
		this.name = "WorkTrackingError";
		this.kind = kind;
		if (details.field !== undefined) this.field = details.field;
		if (details.reason !== undefined) this.reason = details.reason;
		if (details.retryAfterSeconds !== undefined) this.retryAfterSeconds = details.retryAfterSeconds;
		if (details.status !== undefined) this.status = details.status;
	}

	static fromUnknown(rejection: unknown): WorkTrackingError {
		if (isRecord(rejection) && isWorkTrackingErrorKind(rejection.kind)) {
			return new WorkTrackingError(rejection.kind, stringOrUndefined(rejection.message) ?? "", {
				field: stringOrUndefined(rejection.field),
				reason: stringOrUndefined(rejection.reason),
				retryAfterSeconds: finiteNumberOrUndefined(rejection.retryAfterSeconds),
				status: finiteNumberOrUndefined(rejection.status),
			});
		}
		return new WorkTrackingError(WorkTrackingErrorKind.ProviderError, String(rejection));
	}
}
