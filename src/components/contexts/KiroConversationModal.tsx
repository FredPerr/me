import { Modal } from "@mantine/core";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { AgentChat } from "@/components/agent-chat/AgentChat";
import { useKiroAgentSession } from "@/hooks/useKiroAgentSession";
import { createFileProvider } from "@/models/agent-chat/slash/fileProvider";
import type { SlashProvider } from "@/models/agent-chat/slash/SlashProvider";
import { createSteeringProvider } from "@/models/agent-chat/slash/steeringProvider";
import type { KiroConversation } from "@/models/ai-session/KiroConversation";

type KiroConversationModalProps = {
	opened: boolean;
	onClose: () => void;
	contextId: string;
	contextName: string;
	conversation: KiroConversation | undefined;
	isRunning: boolean;
	prompt: string;
	onPromptChange: (prompt: string) => void;
	onSend: (prompt: string) => Promise<void>;
	onKill: () => Promise<void>;
	/** Working directory used to resolve slash-command references, if known. */
	slashBasePath?: string;
};

/**
 * Modal host for a context's Kiro conversation. The chat UI is now the reusable,
 * CLI-agnostic `AgentChat`; this component only adapts the context's Kiro state
 * into an `AgentSession` and supplies the slash-command providers scoped to the
 * context's working directory.
 */
export function KiroConversationModal({
	opened,
	onClose,
	contextId,
	contextName,
	conversation,
	isRunning,
	prompt,
	onPromptChange,
	onSend,
	onKill,
	slashBasePath,
}: KiroConversationModalProps) {
	const { t } = useTranslation();

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
					height: "100vh",
					maxHeight: "100vh",
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
