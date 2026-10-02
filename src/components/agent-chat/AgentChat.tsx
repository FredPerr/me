import { ScrollArea, Stack, Text } from "@mantine/core";
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { useTranslation } from "react-i18next";
import { AgentChatActionsProvider } from "@/components/agent-chat/AgentChatContext";
import { AgentMessageView } from "@/components/agent-chat/AgentMessageView";
import { ChatInput } from "@/components/agent-chat/ChatInput";
import type { PermissionDecision } from "@/models/agent-chat/AgentMessage";
import type { AgentSession } from "@/models/agent-chat/AgentSession";
import type { SlashProvider } from "@/models/agent-chat/slash/SlashProvider";

type AgentChatProps = {
	/** The CLI-agnostic session that drives this conversation. */
	session: AgentSession;
	/** Slash-command sources available in the input. */
	slashProviders?: SlashProvider[];
	/** Controlled prompt value (lets a host seed/prefill the input). */
	prompt: string;
	onPromptChange: (prompt: string) => void;
	/** Max height of the scrollable transcript. */
	transcriptMaxHeight?: number;
};

/**
 * Reusable, CLI-agnostic agent chat. Given an {@link AgentSession}, it renders
 * the conversation (each message's parts via the renderer registry), a prompt
 * input with slash-command autocomplete, and relays permission decisions back
 * through the session. It knows nothing about Kiro or any specific transport —
 * swap the session to drive a different CLI.
 */
export function AgentChat({
	session,
	slashProviders = [],
	prompt,
	onPromptChange,
	transcriptMaxHeight = 440,
}: AgentChatProps) {
	const { t } = useTranslation();

	// Mirror the session's external state into React via useSyncExternalStore so
	// updates from the transport (streamed lines, status) re-render the view.
	const subscribe = useCallback(
		(onStoreChange: () => void) => session.subscribe(onStoreChange),
		[session],
	);
	const state = useSyncExternalStore(subscribe, () => session.getState());

	const [sendError, setSendError] = useState<string | null>(null);
	const viewportRef = useRef<HTMLDivElement>(null);

	const actions = useMemo(
		() => ({
			respondToPermission: (requestId: string, decision: PermissionDecision) => {
				void session.respondToPermission(requestId, decision);
			},
			supportsPermissions: session.supportsPermissions,
		}),
		[session],
	);

	// Follow the tail as content grows.
	const contentSignal = state.messages.reduce((count, message) => count + message.parts.length, 0);
	useEffect(() => {
		if (contentSignal === 0) return;
		viewportRef.current?.scrollTo({ top: viewportRef.current.scrollHeight });
	}, [contentSignal]);

	async function handleSend() {
		const trimmed = prompt.trim();
		if (trimmed.length === 0 || state.isBusy) return;
		setSendError(null);
		try {
			await session.send(trimmed);
			onPromptChange("");
		} catch (error) {
			setSendError(String(error));
		}
	}

	function handleStop() {
		void session.kill();
	}

	return (
		<AgentChatActionsProvider value={actions}>
			<Stack gap="md">
				{state.messages.length === 0 ? (
					<Text size="sm" c="dimmed" py="md" ta="center">
						{t("agentChat.empty")}
					</Text>
				) : (
					<ScrollArea.Autosize mah={transcriptMaxHeight} type="auto" viewportRef={viewportRef}>
						<Stack gap="lg" pr="sm">
							{state.messages.map((message) => (
								<AgentMessageView key={message.id} message={message} />
							))}
						</Stack>
					</ScrollArea.Autosize>
				)}

				<Stack gap="xs">
					<ChatInput
						value={prompt}
						onChange={onPromptChange}
						onSend={handleSend}
						onStop={handleStop}
						isBusy={state.isBusy}
						slashProviders={slashProviders}
					/>
					{sendError && (
						<Text size="xs" c="red.4">
							{sendError}
						</Text>
					)}
					<Text size="xs" c="dimmed">
						{t("agentChat.sendHint")}
					</Text>
				</Stack>
			</Stack>
		</AgentChatActionsProvider>
	);
}
