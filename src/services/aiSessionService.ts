/**
 * Thin service wrapping the AI session Tauri commands and events.
 *
 * Centralizing the `invoke`/`listen` calls here keeps the command names and
 * event wiring in one place. The context kanban drives Kiro runs through this
 * service: spawn a session, listen for its terminal status, and kill it.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
	OutputEvent,
	SpawnedSession,
	SpawnOptions,
	StatusEvent,
} from "@/models/ai-session/AiSession";

/** Event names; must match the Rust constants in `ai_session::mod`. */
const EVENT_OUTPUT = "ai-session://output";
const EVENT_STATUS = "ai-session://status";

export const aiSessionService = {
	/** Spawns a new session and begins streaming its output. */
	spawn(options: SpawnOptions): Promise<SpawnedSession> {
		return invoke<SpawnedSession>("spawn_ai_session", { options });
	},

	/** Terminates a running session by killing its process group. */
	kill(sessionId: string): Promise<void> {
		return invoke<void>("kill_ai_session", { sessionId });
	},

	/** Subscribes to streamed output lines. Returns an unlisten function. */
	onOutput(handler: (event: OutputEvent) => void): Promise<UnlistenFn> {
		return listen<OutputEvent>(EVENT_OUTPUT, (message) => handler(message.payload));
	},

	/** Subscribes to lifecycle status changes. Returns an unlisten function. */
	onStatus(handler: (event: StatusEvent) => void): Promise<UnlistenFn> {
		return listen<StatusEvent>(EVENT_STATUS, (message) => handler(message.payload));
	},
};
