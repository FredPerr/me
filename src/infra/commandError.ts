import type {
	GitCommandError,
	GitHostingCommandError,
	SettingsCommandError,
	WorkspaceCommandError,
} from "@/bindings";

/** Any error a typed command throws. Grows as bounded contexts move to typed commands. */
export type CommandError =
	| GitCommandError
	| GitHostingCommandError
	| SettingsCommandError
	| WorkspaceCommandError;

export function isCommandError(value: unknown): value is CommandError {
	if (typeof value !== "object" || value === null) return false;
	const { kind, message } = value as Record<string, unknown>;
	return typeof kind === "string" && typeof message === "string";
}

/** Text to show for anything caught: a typed command error, a JavaScript error, or a thrown value. */
export function describeError(error: unknown): string {
	if (isCommandError(error)) return error.message;
	if (error instanceof Error) return error.message;
	return String(error);
}
