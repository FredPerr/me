/**
 * Adapts Rust-reported agent runs into the existing {@link KiroConversation}
 * display model, so the conversation modal keeps parsing Kiro `stream-json`
 * output into rich chat parts without change.
 *
 * Rust owns a run's lifecycle and stores its transcript as raw output lines.
 * Here those lines become a {@link KiroTurn}: every raw line is treated as a
 * stdout line (Kiro emits its `stream-json` on stdout), which the existing
 * `kiroConversationToMessages` parser turns into messages. The frontend no
 * longer tags streams, computes folders, or reads the conversation id.
 */

import type { AgentRunStateView, AgentRunView } from "@/bindings";
import type { SessionStatus } from "@/models/ai-session/AiSession";
import type {
	ConversationLine,
	KiroConversation,
	KiroTurn,
} from "@/models/ai-session/KiroConversation";

/** A run still in flight, assembled live from the `agent-run` event stream. */
export type LiveAgentRun = {
	runId: string;
	contextId: string;
	prompt: string;
	lines: string[];
	state: AgentRunStateView;
	startedAt: number;
};

/** Maps a Rust run state to the display status the chat model expects. */
function toSessionStatus(state: AgentRunStateView): SessionStatus {
	switch (state) {
		case "running":
			return "running";
		case "finished":
			return "completed";
		case "failed":
			return "failed";
		case "stopped":
		case "interrupted":
			return "killed";
	}
}

function toConversationLines(outputLines: string[]): ConversationLine[] {
	return outputLines.map((text, index) => ({ id: index, stream: "stdout", text }));
}

function pastRunToTurn(run: AgentRunView): KiroTurn {
	return {
		sessionId: run.id,
		prompt: run.prompt,
		status: toSessionStatus(run.state),
		exitCode: run.exitCode,
		error: null,
		startedAt: Date.parse(run.startedAt),
		lines: toConversationLines(run.outputLines),
	};
}

function liveRunToTurn(run: LiveAgentRun): KiroTurn {
	return {
		sessionId: run.runId,
		prompt: run.prompt,
		status: toSessionStatus(run.state),
		exitCode: null,
		error: null,
		startedAt: run.startedAt,
		lines: toConversationLines(run.lines),
	};
}

/**
 * Builds a context's conversation from its past runs (oldest first) and the
 * live run, if any. A live run replaces its persisted counterpart so a run that
 * has just been recorded does not appear twice while the event stream and the
 * refetched list briefly overlap.
 */
export function buildAgentRunConversation(
	contextId: string,
	pastRuns: AgentRunView[],
	liveRun: LiveAgentRun | undefined,
): KiroConversation | undefined {
	const turns: KiroTurn[] = [];
	for (const run of pastRuns) {
		if (liveRun && run.id === liveRun.runId) continue;
		turns.push(pastRunToTurn(run));
	}
	if (liveRun) turns.push(liveRunToTurn(liveRun));
	if (turns.length === 0) return undefined;
	return { contextId, turns };
}
