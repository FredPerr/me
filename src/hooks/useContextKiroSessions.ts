import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { useAppSettings } from "@/hooks/useAppSettings";
import {
	isTerminalStatus,
	type OutputEvent,
	type StatusEvent,
} from "@/models/ai-session/AiSession";
import { ensurePermissionFloor } from "@/models/ai-session/ensurePermissionFloor";
import type { KiroConversation, KiroTurn } from "@/models/ai-session/KiroConversation";
import type { ContextStatus } from "@/models/ContextStatus";
import type { Context, Project } from "@/models/Project";
import { aiSessionService } from "@/services/aiSessionService";

/** The Kiro adapter id registered in the Rust backend. */
const KIRO_ADAPTER_ID = "kiro";

/** Status a context moves to while a Kiro session runs. */
const RUNNING_STATUS: ContextStatus = "inProgress";
/**
 * Status a context moves to once its Kiro session finishes, whether it
 * completed, failed, or was killed: a human reviews the result either way.
 */
const FINISHED_STATUS: ContextStatus = "needsHuman";

/** Upper bound on buffered output lines per turn to keep memory bounded. */
const MAX_LINES_PER_TURN = 2000;

type RunKiroParams = {
	context: Context;
	prompt: string;
};

type UseContextKiroSessionsResult = {
	/** Spawns a Kiro session for the context and moves it to in progress. */
	runKiro: (params: RunKiroParams) => Promise<void>;
	/** Kills the running session for a context, if any. */
	killKiro: (contextId: string) => Promise<void>;
	/** Whether a Kiro session is currently running for the given context. */
	isRunning: (contextId: string) => boolean;
	/** The conversation transcript for a context, if any runs have happened. */
	getConversation: (contextId: string) => KiroConversation | undefined;
};

/**
 * Resolves the first existing path among the context's IDE path candidates,
 * falling back to the last candidate. Mirrors how "open in IDE" picks the
 * working directory so a Kiro run lands in the same place.
 */
async function resolveWorkingDirectory(project: Project, context: Context): Promise<string> {
	const candidates = project.resolveContextIdePathCandidates(context);
	for (const candidate of candidates) {
		const exists = await invoke<boolean>("check_path_exists", { path: candidate });
		if (exists) return candidate;
	}
	return candidates[candidates.length - 1];
}

/**
 * Drives Kiro CLI runs from the context kanban and keeps a conversation
 * transcript per context. Spawning a session moves its context to "in
 * progress"; when the backend reports the session finished (completed, failed,
 * or killed), the context moves to "needs human" so a person reviews the
 * outcome. Streamed stdout/stderr lines are appended to the matching turn so
 * the UI can show the prompt-and-response conversation live.
 *
 * A single pair of event subscriptions is shared for the hook's lifetime; the
 * session id to context id map lives in a ref so the handlers route events
 * without resubscribing per run.
 */
export function useContextKiroSessions(
	project: Project,
	onStatusChange: (contextId: string, status: ContextStatus) => void,
): UseContextKiroSessionsResult {
	const { settings } = useAppSettings();

	// contextId -> sessionId for sessions currently running.
	const [runningByContext, setRunningByContext] = useState<Record<string, string>>({});
	// contextId -> conversation transcript (all runs for that context).
	const [conversations, setConversations] = useState<Record<string, KiroConversation>>({});

	// Reverse lookup sessionId -> contextId, read inside the event handlers.
	const sessionToContext = useRef<Record<string, string>>({});
	// Monotonic counter for stable, unique line ids across all turns.
	const lineCounter = useRef(0);
	// Latest status-change callback, read without resubscribing.
	const onStatusChangeRef = useRef(onStatusChange);
	onStatusChangeRef.current = onStatusChange;

	const appendLine = useCallback((event: OutputEvent) => {
		const contextId = sessionToContext.current[event.sessionId];
		if (!contextId) return;
		lineCounter.current += 1;
		const lineId = lineCounter.current;

		setConversations((current) => {
			const conversation = current[contextId];
			if (!conversation) return current;
			const turns = conversation.turns.map((turn) => {
				if (turn.sessionId !== event.sessionId) return turn;
				const lines = [...turn.lines, { id: lineId, stream: event.stream, text: event.line }];
				const trimmed =
					lines.length > MAX_LINES_PER_TURN
						? lines.slice(lines.length - MAX_LINES_PER_TURN)
						: lines;
				return { ...turn, lines: trimmed };
			});
			return { ...current, [contextId]: { ...conversation, turns } };
		});
	}, []);

	const applyStatus = useCallback((event: StatusEvent) => {
		const contextId = sessionToContext.current[event.sessionId];
		if (!contextId) return;

		setConversations((current) => {
			const conversation = current[contextId];
			if (!conversation) return current;
			const turns = conversation.turns.map((turn) =>
				turn.sessionId === event.sessionId
					? { ...turn, status: event.status, exitCode: event.exitCode, error: event.error }
					: turn,
			);
			return { ...current, [contextId]: { ...conversation, turns } };
		});

		if (!isTerminalStatus(event.status)) return;

		onStatusChangeRef.current(contextId, FINISHED_STATUS);
		delete sessionToContext.current[event.sessionId];
		setRunningByContext((current) => {
			const next = { ...current };
			delete next[contextId];
			return next;
		});
	}, []);

	useEffect(() => {
		let disposed = false;
		const unlisteners: Array<() => void> = [];
		const track = (fn: () => void) => {
			if (disposed) fn();
			else unlisteners.push(fn);
		};

		aiSessionService.onOutput(appendLine).then(track);
		aiSessionService.onStatus(applyStatus).then(track);

		return () => {
			disposed = true;
			for (const unlisten of unlisteners) unlisten();
		};
	}, [appendLine, applyStatus]);

	const runKiro = useCallback(
		async ({ context, prompt }: RunKiroParams) => {
			const workingDirectory = await resolveWorkingDirectory(project, context);
			// Seed the safety deny floor before a trusted run, if not already present.
			await ensurePermissionFloor(workingDirectory);
			const spawned = await aiSessionService.spawn({
				adapterId: KIRO_ADAPTER_ID,
				prompt,
				workingDirectory,
				shell: settings?.shell || undefined,
				trustAllTools: true,
			});

			sessionToContext.current[spawned.sessionId] = context.id;

			const turn: KiroTurn = {
				sessionId: spawned.sessionId,
				prompt,
				status: "running",
				exitCode: null,
				error: null,
				startedAt: Date.now(),
				lines: [],
			};
			setConversations((current) => {
				const existing = current[context.id];
				const turns = existing ? [...existing.turns, turn] : [turn];
				return { ...current, [context.id]: { contextId: context.id, turns } };
			});
			setRunningByContext((current) => ({ ...current, [context.id]: spawned.sessionId }));
			onStatusChangeRef.current(context.id, RUNNING_STATUS);
		},
		[project, settings?.shell],
	);

	const killKiro = useCallback(
		async (contextId: string) => {
			const sessionId = runningByContext[contextId];
			if (!sessionId) return;
			await aiSessionService.kill(sessionId);
		},
		[runningByContext],
	);

	const isRunning = useCallback(
		(contextId: string) => Boolean(runningByContext[contextId]),
		[runningByContext],
	);

	const getConversation = useCallback(
		(contextId: string) => conversations[contextId],
		[conversations],
	);

	return { runKiro, killKiro, isRunning, getConversation };
}
