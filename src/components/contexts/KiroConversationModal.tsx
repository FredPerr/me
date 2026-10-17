import { Modal } from "@mantine/core";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { AgentChat } from "@/components/agent-chat/AgentChat";
import { useContextAgentRunConversation } from "@/hooks/agent-runs/useContextAgentRunConversation";
import { useKiroAgentSession } from "@/hooks/useKiroAgentSession";
import { createFileProvider } from "@/models/agent-chat/slash/fileProvider";
import type { SlashProvider } from "@/models/agent-chat/slash/SlashProvider";
import { createSteeringProvider } from "@/models/agent-chat/slash/steeringProvider";
import type { LiveAgentRun } from "@/models/agent-runs/agentRunConversation";

type KiroConversationModalProps = {
	opened: boolean;
	onClose: () => void;
	contextId: string;
	contextName: string;
	/** The run streaming for this context right now, if any. */
	liveRun: LiveAgentRun | undefined;
	isRunning: boolean;
	prompt: string;
	onPromptChange: (prompt: string) => void;
	onSend: (prompt: string) => Promise<void>;
	onKill: () => Promise<void>;
	/** Working directory used to resolve slash-command references, if known. */
	slashBasePath?: string;
};

/**
 * Modal host for a context's agent conversation. The transcript is the context's
 * recorded runs read from Rust (surviving reloads) merged with the live run from
 * the `agent-run` event stream. The chat UI is the reusable, CLI-agnostic
 * `AgentChat`; this component adapts the run state into an `AgentSession` and
 * supplies the slash-command providers scoped to the context's working directory.
 */
export function KiroConversationModal({
	opened,
	onClose,
	contextId,
	contextName,
	liveRun,
	isRunning,
	prompt,
	onPromptChange,
	onSend,
	onKill,
	slashBasePath,
}: KiroConversationModalProps) {
	const { t } = useTranslation();

	const conversation = useContextAgentRunConversation(contextId, liveRun);

	const session = useKiroAgentSession({
		id: contextId,
		conversation,
		isRunning,
		onSend,
		onKill,
	});

	const slashProviders = useMemo<SlashProvider[]>(() => {
		if (!slashBasePath) return [];
		return [createSteeringProvider(slashBasePath), createFileProvider(slashBasePath)];
	}, [slashBasePath]);

	return (
		<Modal
			opened={opened}
			onClose={onClose}
			title={t("contexts.kiro.conversation.title", { name: contextName })}
			size="xl"
			closeOnClickOutside={false}
			closeOnEscape={false}
			styles={{
				content: {
					// Fill the viewport minus the modal's own top/bottom offset so the
					// content never overflows past the bottom of the screen.
					height: "calc(100dvh - 2 * var(--modal-y-offset, 5dvh))",
					maxHeight: "calc(100dvh - 2 * var(--modal-y-offset, 5dvh))",
					display: "flex",
					flexDirection: "column",
				},
				body: {
					flex: 1,
					minHeight: 0,
					display: "flex",
					flexDirection: "column",
				},
			}}
		>
			<AgentChat
				session={session}
				slashProviders={slashProviders}
				prompt={prompt}
				onPromptChange={onPromptChange}
				fill
			/>
		</Modal>
	);
}
