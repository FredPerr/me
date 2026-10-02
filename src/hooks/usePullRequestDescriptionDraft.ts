import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { useAppSettings } from "@/hooks/useAppSettings";
import { isTerminalStatus } from "@/models/ai-session/AiSession";
import { extractAgentText } from "@/models/ai-session/acp/acpEvents";
import { looksLikeAcpStream } from "@/models/ai-session/acp/acpStreamToParts";
import { ensurePermissionFloor } from "@/models/ai-session/ensurePermissionFloor";
import type { Context, Project, Repository } from "@/models/Project";
import { buildPullRequestDescriptionPrompt } from "@/models/pull-request/pullRequestDescriptionPrompt";
import { readPullRequestTemplate } from "@/models/pull-request/pullRequestTemplate";
import { aiSessionService } from "@/services/aiSessionService";

/** The Kiro adapter id registered in the Rust backend. */
const KIRO_ADAPTER_ID = "kiro";

/** Phase of a single per-repository generation run. */
type DescriptionDraftStatus = "idle" | "generating" | "ready" | "error";

/** Per-repository generation state, keyed by repository id. */
type RepositoryDraftState = {
	status: DescriptionDraftStatus;
	description: string;
	error: string | null;
};

const INITIAL_STATE: RepositoryDraftState = {
	status: "idle",
	description: "",
	error: null,
};

type GenerateInput = {
	context: Context;
	repository: Repository;
	/** Branch the pull request targets; the diff is computed against it. */
	baseBranch: string;
};

type UsePullRequestDescriptionDraftResult = {
	/** Generation state for a repository; defaults to idle. */
	stateFor: (repositoryId: string) => RepositoryDraftState;
	/** Spawn a headless Kiro run that writes the description for one repo. */
	generate: (input: GenerateInput) => Promise<void>;
	/** Called when the user accepts/overwrites a generated description. */
	consume: (repositoryId: string) => void;
};

/**
 * Drives one headless Kiro run per repository to write a pull request
 * description from that repository's diff (branch vs. base context) and its
 * `.github` PR template. Mirrors {@link useBulkContextDraft}: it buffers the
 * run's stdout and, on terminal status, reconstructs the agent's answer from
 * the ACP stream. Nothing is persisted here — the modal owns the drafts and
 * decides what to do with the generated text.
 */
export function usePullRequestDescriptionDraft(
	project: Project,
): UsePullRequestDescriptionDraftResult {
	const { settings } = useAppSettings();

	const [statesByRepository, setStatesByRepository] = useState<
		Record<string, RepositoryDraftState>
	>({});

	// Maps the active session id to the repository it generates for, so the
	// shared output/status listeners can route events to the right repository.
	const sessionToRepository = useRef<Map<string, string>>(new Map());
	// Buffered stdout per active session id, joined and parsed on completion.
	const outputBuffers = useRef<Map<string, string[]>>(new Map());

	const setRepositoryState = useCallback(
		(repositoryId: string, next: Partial<RepositoryDraftState>) => {
			setStatesByRepository((current) => ({
				...current,
				[repositoryId]: { ...(current[repositoryId] ?? INITIAL_STATE), ...next },
			}));
		},
		[],
	);

	useEffect(() => {
		let disposed = false;
		const unlisteners: Array<() => void> = [];
		const track = (unlisten: () => void) => {
			if (disposed) unlisten();
			else unlisteners.push(unlisten);
		};

		aiSessionService
			.onOutput((event) => {
				if (!sessionToRepository.current.has(event.sessionId)) return;
				if (event.stream !== "stdout") return;
				const buffer = outputBuffers.current.get(event.sessionId) ?? [];
				buffer.push(event.line);
				outputBuffers.current.set(event.sessionId, buffer);
			})
			.then(track);

		aiSessionService
			.onStatus((event) => {
				const repositoryId = sessionToRepository.current.get(event.sessionId);
				if (!repositoryId) return;
				if (!isTerminalStatus(event.status)) return;

				const lines = outputBuffers.current.get(event.sessionId) ?? [];
				sessionToRepository.current.delete(event.sessionId);
				outputBuffers.current.delete(event.sessionId);

				if (event.status === "failed" || event.status === "killed") {
					setRepositoryState(repositoryId, {
						status: "error",
						error: event.error ?? `Kiro session ${event.status}`,
					});
					return;
				}

				const agentText = looksLikeAcpStream(lines) ? extractAgentText(lines) : lines.join("\n");
				const description = agentText.trim();

				if (!description) {
					setRepositoryState(repositoryId, {
						status: "error",
						error: "The generator returned an empty description.",
					});
					return;
				}

				setRepositoryState(repositoryId, { status: "ready", description, error: null });
			})
			.then(track);

		return () => {
			disposed = true;
			for (const unlisten of unlisteners) unlisten();
		};
	}, [setRepositoryState]);

	const generate = useCallback(
		async ({ context, repository, baseBranch }: GenerateInput) => {
			const repositoryId = repository.id;
			setRepositoryState(repositoryId, { status: "generating", error: null });

			const worktreePath = context.getWorktreePath(project.path, repository);

			try {
				const [diff, template] = await Promise.all([
					invoke<string>("get_worktree_diff", { path: worktreePath, baseBranch }),
					readPullRequestTemplate(worktreePath),
				]);

				if (diff.trim().length === 0) {
					setRepositoryState(repositoryId, {
						status: "error",
						error: "No changes found against the base branch.",
					});
					return;
				}

				const prompt = buildPullRequestDescriptionPrompt({
					contextName: context.name,
					repositoryName: repository.name,
					diff,
					template,
				});

				await ensurePermissionFloor(worktreePath);
				const spawned = await aiSessionService.spawn({
					adapterId: KIRO_ADAPTER_ID,
					prompt,
					workingDirectory: worktreePath,
					shell: settings?.shell || undefined,
					trustAllTools: true,
				});

				sessionToRepository.current.set(spawned.sessionId, repositoryId);
				outputBuffers.current.set(spawned.sessionId, []);
			} catch (error) {
				setRepositoryState(repositoryId, { status: "error", error: String(error) });
			}
		},
		[project, settings?.shell, setRepositoryState],
	);

	const consume = useCallback(
		(repositoryId: string) => {
			setRepositoryState(repositoryId, { status: "idle", description: "" });
		},
		[setRepositoryState],
	);

	const stateFor = useCallback(
		(repositoryId: string): RepositoryDraftState =>
			statesByRepository[repositoryId] ?? INITIAL_STATE,
		[statesByRepository],
	);

	return { stateFor, generate, consume };
}
