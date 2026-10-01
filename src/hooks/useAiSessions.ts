import { useCallback, useEffect, useRef, useState } from "react";
import type {
	AdapterInfo,
	AiSession,
	LogLine,
	OutputEvent,
	SpawnOptions,
	StatusEvent,
} from "@/models/ai-session/AiSession";
import { aiSessionService } from "@/services/aiSessionService";

/** Upper bound on buffered log lines per session to keep memory bounded. */
const MAX_LOG_LINES = 2000;

type UseAiSessionsResult = {
	sessions: AiSession[];
	adapters: AdapterInfo[];
	loadingAdapters: boolean;
	spawn: (options: SpawnOptions, prompt: string) => Promise<void>;
	kill: (sessionId: string) => Promise<void>;
	clear: (sessionId: string) => void;
};

function appendLogLine(session: AiSession, nextId: number, event: OutputEvent): AiSession {
	const line: LogLine = { id: nextId, stream: event.stream, text: event.line };
	const logs = [...session.logs, line];
	const trimmed = logs.length > MAX_LOG_LINES ? logs.slice(logs.length - MAX_LOG_LINES) : logs;
	return { ...session, logs: trimmed };
}

/**
 * Orchestrates AI CLI sessions from React: loads available adapters, spawns and
 * kills sessions, and keeps live status + streamed output in sync by listening
 * to the backend's Tauri events. A single subscription is shared for the hook's
 * lifetime; the sessions map is the source of truth for the UI.
 */
export function useAiSessions(): UseAiSessionsResult {
	const [sessions, setSessions] = useState<Record<string, AiSession>>({});
	const [adapters, setAdapters] = useState<AdapterInfo[]>([]);
	const [loadingAdapters, setLoadingAdapters] = useState(true);

	// Monotonic counter for stable, unique log line ids across all sessions.
	const logLineCounter = useRef(0);
	// Latest adapters, read inside spawn without making it depend on adapters.
	const adaptersRef = useRef<AdapterInfo[]>([]);
	adaptersRef.current = adapters;

	const applyOutput = useCallback((event: OutputEvent) => {
		setSessions((current) => {
			const session = current[event.sessionId];
			if (!session) return current;
			logLineCounter.current += 1;
			return {
				...current,
				[event.sessionId]: appendLogLine(session, logLineCounter.current, event),
			};
		});
	}, []);

	const applyStatus = useCallback((event: StatusEvent) => {
		setSessions((current) => {
			const session = current[event.sessionId];
			// Status may arrive before spawn() resolves; ignore until the session
			// is registered locally, where its prompt/metadata lives.
			if (!session) return current;
			return {
				...current,
				[event.sessionId]: {
					...session,
					status: event.status,
					pid: event.pid ?? session.pid,
					exitCode: event.exitCode,
					error: event.error,
				},
			};
		});
	}, []);

	useEffect(() => {
		aiSessionService
			.listAdapters()
			.then(setAdapters)
			.catch(() => setAdapters([]))
			.finally(() => setLoadingAdapters(false));
	}, []);

	useEffect(() => {
		const unlisteners: Array<() => void> = [];
		let disposed = false;

		aiSessionService.onOutput(applyOutput).then((unlisten) => {
			if (disposed) unlisten();
			else unlisteners.push(unlisten);
		});
		aiSessionService.onStatus(applyStatus).then((unlisten) => {
			if (disposed) unlisten();
			else unlisteners.push(unlisten);
		});

		return () => {
			disposed = true;
			for (const unlisten of unlisteners) unlisten();
		};
	}, [applyOutput, applyStatus]);

	const spawn = useCallback(async (options: SpawnOptions, prompt: string) => {
		const spawned = await aiSessionService.spawn(options);
		const adapterName =
			adaptersRef.current.find((adapter) => adapter.id === spawned.adapterId)?.displayName ??
			spawned.adapterId;
		const session: AiSession = {
			sessionId: spawned.sessionId,
			adapterId: spawned.adapterId,
			adapterName,
			workingDirectory: spawned.workingDirectory,
			prompt,
			pid: spawned.pid,
			status: "running",
			exitCode: null,
			error: null,
			startedAt: Date.now(),
			logs: [],
		};
		setSessions((current) => ({ ...current, [session.sessionId]: session }));
	}, []);

	const kill = useCallback(async (sessionId: string) => {
		await aiSessionService.kill(sessionId);
	}, []);

	const clear = useCallback((sessionId: string) => {
		setSessions((current) => {
			const next = { ...current };
			delete next[sessionId];
			return next;
		});
	}, []);

	const orderedSessions = Object.values(sessions).sort((a, b) => b.startedAt - a.startedAt);

	return {
		sessions: orderedSessions,
		adapters,
		loadingAdapters,
		spawn,
		kill,
		clear,
	};
}
