/**
 * Frontend types for AI CLI session orchestration.
 *
 * These mirror the Rust wire types in `src-tauri/src/ai_session` (serialized as
 * camelCase). They are kept in sync by hand, matching the repo convention of
 * hand-mirroring Rust structs (see `usePushWorktree`'s `PushResult`).
 */

/** Lifecycle state of a session; mirrors Rust `SessionStatus`. */
export type SessionStatus = "running" | "completed" | "failed" | "killed";

/** Which stream a line of output came from; mirrors Rust `OutputStream`. */
export type OutputStream = "stdout" | "stderr";

/** A streamed line of output; mirrors Rust `OutputEvent`. */
export type OutputEvent = {
	sessionId: string;
	stream: OutputStream;
	line: string;
};

/** A lifecycle transition; mirrors Rust `StatusEvent`. */
export type StatusEvent = {
	sessionId: string;
	adapterId: string;
	status: SessionStatus;
	pid: number | null;
	exitCode: number | null;
	error: string | null;
};

/** A supported AI CLI adapter; mirrors Rust `AdapterInfo`. */
export type AdapterInfo = {
	id: string;
	displayName: string;
	defaultCommand: string;
};

/** The descriptor returned when a session starts; mirrors Rust `SpawnedSession`. */
export type SpawnedSession = {
	sessionId: string;
	adapterId: string;
	pid: number | null;
	workingDirectory: string;
};

/** A single environment variable pair; mirrors Rust `EnvVar`. */
export type EnvVar = {
	key: string;
	value: string;
};

/** Options for spawning a session; mirrors Rust `SpawnOptions` (camelCase). */
export type SpawnOptions = {
	adapterId: string;
	prompt: string;
	workingDirectory: string;
	command?: string;
	agent?: string;
	shell?: string;
	environment?: EnvVar[];
};

/** A single buffered output line held in the UI, with a stable id for keying. */
export type LogLine = {
	id: number;
	stream: OutputStream;
	text: string;
};

/**
 * A session as tracked in the UI: the backend descriptor plus the live status
 * and buffered output. Kept as a plain readonly-friendly shape (updated via
 * immutable copies in the hook's reducer) rather than a class, since it is
 * ephemeral UI state rather than a persisted domain aggregate.
 */
export type AiSession = {
	sessionId: string;
	adapterId: string;
	adapterName: string;
	workingDirectory: string;
	prompt: string;
	pid: number | null;
	status: SessionStatus;
	exitCode: number | null;
	error: string | null;
	startedAt: number;
	logs: LogLine[];
};

/** Terminal statuses for which no further output or kill action applies. */
const TERMINAL_STATUSES: ReadonlySet<SessionStatus> = new Set<SessionStatus>([
	"completed",
	"failed",
	"killed",
]);

export function isTerminalStatus(status: SessionStatus): boolean {
	return TERMINAL_STATUSES.has(status);
}

export function isSessionActive(session: AiSession): boolean {
	return !isTerminalStatus(session.status);
}
