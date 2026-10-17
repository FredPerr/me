/**
 * Frontend types for AI CLI session orchestration.
 *
 * The frontend no longer spawns a session: AI sessions are driven only by Rust
 * use cases (agent runs and AI assists). These types mirror the plain
 * `ai-session://output` and `ai-session://status` Tauri events, which are not
 * tauri-specta events, so they stay hand-kept and in sync by hand.
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

/** Terminal statuses for which no further output or kill action applies. */
const TERMINAL_STATUSES: ReadonlySet<SessionStatus> = new Set<SessionStatus>([
	"completed",
	"failed",
	"killed",
]);

export function isTerminalStatus(status: SessionStatus): boolean {
	return TERMINAL_STATUSES.has(status);
}
