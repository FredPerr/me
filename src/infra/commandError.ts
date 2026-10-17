import type {
	GitCommandError,
	GitHostingCommandError,
	SettingsCommandError,
	WorkspaceCommandError,
	WorkTrackingCommandError,
} from "@/bindings";

/** Any error a typed command throws. Grows as bounded contexts move to typed commands. */
export type CommandError =
	| GitCommandError
	| GitHostingCommandError
	| SettingsCommandError
	| WorkspaceCommandError
	| WorkTrackingCommandError;

export function isCommandError(value: unknown): value is CommandError {
	if (typeof value !== "object" || value === null) return false;
	const { kind, message } = value as Record<string, unknown>;
	return typeof kind === "string" && typeof message === "string";
}

const WORK_TRACKING_ERROR_KINDS: ReadonlySet<string> = new Set([
	"notConfigured",
	"invalidInput",
	"unauthorized",
	"forbidden",
	"notFound",
	"rateLimited",
	"timeout",
	"network",
	"providerError",
	"invalidResponse",
	"storageError",
]);

/** A failed work-tracking command, recognised by its error kind. */
export function isWorkTrackingCommandError(value: unknown): value is WorkTrackingCommandError {
	return isCommandError(value) && WORK_TRACKING_ERROR_KINDS.has(value.kind);
}

/** A caught value as a work-tracking command error, defaulting unknowns to `providerError`. */
export function toWorkTrackingCommandError(value: unknown): WorkTrackingCommandError {
	if (isWorkTrackingCommandError(value)) return value;
	return {
		kind: "providerError",
		message: describeError(value),
		field: null,
		reason: null,
		retryAfterSeconds: null,
		status: null,
	};
}

/** Text to show for anything caught: a typed command error, a JavaScript error, or a thrown value. */
export function describeError(error: unknown): string {
	if (isCommandError(error)) return error.message;
	if (error instanceof Error) return error.message;
	return String(error);
}
