import {
	ActionIcon,
	Box,
	Button,
	Group,
	Loader,
	Modal,
	Paper,
	ScrollArea,
	Stack,
	Text,
} from "@mantine/core";
import { PaperPlaneTiltIcon, RobotIcon, StopCircleIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
	ConversationLine,
	KiroConversation,
	KiroTurn,
} from "@/models/ai-session/KiroConversation";

type KiroConversationModalProps = {
	opened: boolean;
	onClose: () => void;
	contextName: string;
	conversation: KiroConversation | undefined;
	isRunning: boolean;
	prompt: string;
	onPromptChange: (prompt: string) => void;
	onSend: (prompt: string) => Promise<void>;
	onKill: () => Promise<void>;
};

function AgentOutput({ lines }: { lines: ConversationLine[] }) {
	const { t } = useTranslation();

	if (lines.length === 0) {
		return (
			<Text size="xs" c="dimmed" ff="monospace">
				{t("contexts.kiro.conversation.waiting")}
			</Text>
		);
	}

	return (
		<Box
			ff="monospace"
			fz="xs"
			style={{ whiteSpace: "pre-wrap", wordBreak: "break-word", lineHeight: 1.5 }}
		>
			{lines.map((line) => (
				<Text
					key={line.id}
					component="div"
					ff="monospace"
					fz="xs"
					c={line.stream === "stderr" ? "red.4" : undefined}
				>
					{line.text}
				</Text>
			))}
		</Box>
	);
}

/**
 * A single run rendered as a chat exchange: the user's prompt in a gray bubble
 * aligned to the right, and the agent's response in a bordered panel. While the
 * agent is running, a spinner replaces its avatar; otherwise the Kiro face.
 */
function ConversationTurn({ turn }: { turn: KiroTurn }) {
	const isRunning = turn.status === "running";

	return (
		<Stack gap={8}>
			<Group justify="flex-end">
				<Paper bg="dark.5" px="sm" py={6} radius="lg" style={{ maxWidth: "80%" }}>
					<Text size="sm" style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
						{turn.prompt}
					</Text>
				</Paper>
			</Group>
			<Group gap="xs" align="flex-start" wrap="nowrap">
				{isRunning ? (
					<Loader size={16} color="grape" />
				) : (
					<RobotIcon size={16} color="var(--mantine-color-grape-4)" />
				)}
				<Paper
					withBorder
					px="sm"
					py={6}
					radius="md"
					style={{ flex: 1, minWidth: 0, borderColor: "var(--mantine-color-dark-4)" }}
				>
					<Stack gap={4}>
						{turn.error && (
							<Text size="xs" c="red.4">
								{turn.error}
							</Text>
						)}
						<AgentOutput lines={turn.lines} />
					</Stack>
				</Paper>
			</Group>
		</Stack>
	);
}

/**
 * Shows the full prompt-and-response conversation for a context's Kiro runs and
 * lets the user send another prompt or stop a running one. Output streams in
 * live via the parent hook's event subscription; this view just renders and
 * follows the tail.
 */
export function KiroConversationModal({
	opened,
	onClose,
	contextName,
	conversation,
	isRunning,
	prompt,
	onPromptChange,
	onSend,
	onKill,
}: KiroConversationModalProps) {
	const { t } = useTranslation();
	const [sending, setSending] = useState(false);
	const [killing, setKilling] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const viewportRef = useRef<HTMLDivElement>(null);

	const turns = conversation?.turns ?? [];
	// A single growth signal: total lines across turns plus the turn count.
	// Referenced inside the effect so it genuinely drives the auto-scroll.
	const contentSignal = turns.reduce((count, turn) => count + turn.lines.length, turns.length);

	useEffect(() => {
		if (contentSignal === 0) return;
		const viewport = viewportRef.current;
		if (viewport) {
			viewport.scrollTo({ top: viewport.scrollHeight });
		}
	}, [contentSignal]);

	async function handleSend() {
		const trimmed = prompt.trim();
		if (!trimmed || isRunning) return;
		setSending(true);
		setError(null);
		try {
			await onSend(trimmed);
			onPromptChange("");
		} catch (sendError) {
			setError(String(sendError));
		} finally {
			setSending(false);
		}
	}

	async function handleKill() {
		setKilling(true);
		try {
			await onKill();
		} finally {
			setKilling(false);
		}
	}

	function handleKeyDown(event: React.KeyboardEvent<HTMLTextAreaElement>) {
		if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			handleSend();
		}
	}

	return (
		<Modal
			opened={opened}
			onClose={onClose}
			title={t("contexts.kiro.conversation.title", { name: contextName })}
			size="xl"
			closeOnClickOutside={false}
			closeOnEscape={false}
		>
			<Stack gap="md">
				{turns.length === 0 ? (
					<Text size="sm" c="dimmed" py="md" ta="center">
						{t("contexts.kiro.conversation.empty")}
					</Text>
				) : (
					<ScrollArea.Autosize mah={420} type="auto" viewportRef={viewportRef}>
						<Stack gap="lg" pr="sm">
							{turns.map((turn) => (
								<ConversationTurn key={turn.sessionId} turn={turn} />
							))}
						</Stack>
					</ScrollArea.Autosize>
				)}

				<Stack gap="xs">
					<textarea
						value={prompt}
						onChange={(event) => onPromptChange(event.currentTarget.value)}
						onKeyDown={handleKeyDown}
						placeholder={t("contexts.kiro.conversation.inputPlaceholder")}
						disabled={isRunning || sending}
						rows={8}
						style={{
							width: "100%",
							minHeight: 160,
							resize: "vertical",
							fontFamily: "inherit",
							fontSize: "var(--mantine-font-size-sm)",
							padding: "8px 10px",
							borderRadius: "var(--mantine-radius-sm)",
							border: "1px solid var(--mantine-color-dark-4)",
							background: "var(--mantine-color-dark-6)",
							color: "inherit",
						}}
					/>
					{error && (
						<Text size="xs" c="red.4">
							{error}
						</Text>
					)}
					<Group justify="space-between">
						<Text size="xs" c="dimmed">
							{t("contexts.kiro.conversation.sendHint")}
						</Text>
						<Group gap="xs">
							{isRunning && (
								<Button
									variant="light"
									color="red"
									size="xs"
									leftSection={<StopCircleIcon size={16} />}
									loading={killing}
									onClick={handleKill}
								>
									{t("contexts.kiro.stop")}
								</Button>
							)}
							<Group gap={4} wrap="nowrap">
								<ActionIcon
									size="lg"
									variant="filled"
									disabled={isRunning || sending || prompt.trim().length === 0}
									loading={sending}
									onClick={handleSend}
									aria-label={t("contexts.kiro.conversation.send")}
								>
									<PaperPlaneTiltIcon size={16} />
								</ActionIcon>
							</Group>
						</Group>
					</Group>
				</Stack>
			</Stack>
		</Modal>
	);
}
