import type { GitCommandError } from "@/bindings";

/** Any error a typed command throws. Grows as bounded contexts move to typed commands. */
export type CommandError = GitCommandError;

export function isCommandError(value: unknown): value is CommandError {
	if (typeof value !== "object" || value === null) return false;
	const { kind, message } = value as Record<string, unknown>;
	return typeof kind === "string" && typeof message === "string";
}
