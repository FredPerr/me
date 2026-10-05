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

/** The descriptor returned when a session starts; mirrors Rust `SpawnedSession`. */
export type SpawnedSession = {
	sessionId: string;
	adapterId: string;
	pid: number | null;
	workingDirectory: string;
};

/** A single environment variable pair; mirrors Rust `EnvVar`. */
type EnvVar = {
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
	/** Prior Kiro conversation id to resume so a follow-up keeps the context. */
	resumeId?: string;
	shell?: string;
	environment?: EnvVar[];
	/** Run tools without per-action confirmation (headless runs set this). */
	trustAllTools?: boolean;
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
