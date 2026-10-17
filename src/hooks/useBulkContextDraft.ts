import { useCallback, useEffect, useRef, useState } from "react";
import { commands, type ProjectView } from "@/bindings";
import { useAppSettings } from "@/hooks/useAppSettings";
import { isTerminalStatus } from "@/models/ai-session/AiSession";
import { extractAgentText } from "@/models/ai-session/acp/acpEvents";
import { looksLikeAcpStream } from "@/models/ai-session/acp/acpStreamToParts";
import { ensurePermissionFloor } from "@/models/ai-session/ensurePermissionFloor";
import {
	type BulkContextDraft,
	type BulkDraftParseError,
	parseBulkContextDrafts,
} from "@/models/bulk/BulkContextDraft";
import { buildBulkContextPrompt } from "@/models/bulk/bulkContextPrompt";
import { aiSessionService } from "@/services/aiSessionService";

/** The Kiro adapter id registered in the Rust backend. */
const KIRO_ADAPTER_ID = "kiro";

/** Phase of the one-shot bulk generation run. */
export type BulkDraftStatus = "idle" | "generating" | "ready" | "error";

export type BulkDraftError =
	| { kind: "parse"; reason: BulkDraftParseError }
	| { kind: "session"; message: string };

type UseBulkContextDraftResult = {
	status: BulkDraftStatus;
	drafts: BulkContextDraft[];
	error: BulkDraftError | null;
	/** Spawns a headless Kiro run that expands the instruction into drafts. */
	generate: (instruction: string) => Promise<void>;
	/** Clears drafts/error and returns to the idle instruction phase. */
	reset: () => void;
};

/** Names each draft's branch under the project's naming; drafts it can't name are dropped. */
async function nameDraftBranches(
	project: ProjectView,
	drafts: readonly BulkContextDraft[],
): Promise<BulkContextDraft[]> {
	const named = await Promise.all(
		drafts.map(async (draft) => {
			try {
				const branchName = await commands.workspaceSuggestBranchName(
					project.tag,
					draft.branchName,
					project.branchNaming.defaultPrefix,
				);
				return [{ ...draft, branchName }];
			} catch {
				return [];
			}
		}),
	);
	return named.flat();
}

/**
 * Drives a single headless Kiro run that expands one high-level instruction
 * into a list of context drafts. Unlike a context agent run, this run is
 * not tied to a context and nothing is persisted: it runs at the project root,
 * buffers stdout, and on terminal status parses the output into drafts whose
 * branch names the project then names (rows it can't name are dropped).
 *
 * Output and status subscriptions live for the hook's lifetime; the active
 * session id is held in a ref so the handlers only react to the current run.
 */
export function useBulkContextDraft(project: ProjectView): UseBulkContextDraftResult {
	const { settings } = useAppSettings();

	const [status, setStatus] = useState<BulkDraftStatus>("idle");
	const [drafts, setDrafts] = useState<BulkContextDraft[]>([]);
	const [error, setError] = useState<BulkDraftError | null>(null);

	// Session id of the run in flight; null when none is active.
	const activeSessionId = useRef<string | null>(null);
	// Buffered stdout for the active run, joined and parsed on terminal status.
	const outputBuffer = useRef<string[]>([]);
	// Latest project, read inside the status handler without resubscribing.
	const projectRef = useRef(project);
	projectRef.current = project;

	useEffect(() => {
		let disposed = false;
		const unlisteners: Array<() => void> = [];
		const track = (unlisten: () => void) => {
			if (disposed) unlisten();
			else unlisteners.push(unlisten);
		};

		aiSessionService
			.onOutput((event) => {
				if (event.sessionId !== activeSessionId.current) return;
				if (event.stream === "stdout") outputBuffer.current.push(event.line);
			})
			.then(track);

		aiSessionService
			.onStatus((event) => {
				if (event.sessionId !== activeSessionId.current) return;
				if (!isTerminalStatus(event.status)) return;

				activeSessionId.current = null;

				if (event.status === "failed" || event.status === "killed") {
					setStatus("error");
					setError({
						kind: "session",
						message: event.error ?? `Kiro session ${event.status}`,
					});
					return;
				}

				// Kiro streams ACP JSON lines; reconstruct the agent's answer text
				// (falling back to the raw buffer if the output was plain text).
				const lines = outputBuffer.current;
				const agentText = looksLikeAcpStream(lines) ? extractAgentText(lines) : lines.join("\n");
				const result = parseBulkContextDrafts(agentText);
				if (!result.ok) {
					setStatus("error");
					setError({ kind: "parse", reason: result.error });
					return;
				}

				nameDraftBranches(projectRef.current, result.drafts).then((namedDrafts) => {
					if (namedDrafts.length === 0) {
						setStatus("error");
						setError({ kind: "parse", reason: "no-valid-rows" });
						return;
					}
					setDrafts(namedDrafts);
					setStatus("ready");
				});
			})
			.then(track);

		return () => {
			disposed = true;
			for (const unlisten of unlisteners) unlisten();
		};
	}, []);

	const generate = useCallback(
		async (instruction: string) => {
			setStatus("generating");
			setError(null);
			setDrafts([]);
			outputBuffer.current = [];

			const workingDirectory = project.path;

			try {
				await ensurePermissionFloor(workingDirectory);
				const spawned = await aiSessionService.spawn({
					adapterId: KIRO_ADAPTER_ID,
					prompt: buildBulkContextPrompt(instruction),
					workingDirectory,
					shell: settings?.shell || undefined,
					trustAllTools: true,
				});
				activeSessionId.current = spawned.sessionId;
			} catch (spawnError) {
				setStatus("error");
				setError({ kind: "session", message: String(spawnError) });
			}
		},
		[project, settings?.shell],
	);

	const reset = useCallback(() => {
		activeSessionId.current = null;
		outputBuffer.current = [];
		setDrafts([]);
		setError(null);
		setStatus("idle");
	}, []);

	return { status, drafts, error, generate, reset };
}

export type { BulkContextDraft };
