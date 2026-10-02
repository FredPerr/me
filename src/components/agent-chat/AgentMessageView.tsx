import { Group, Loader, Paper, Stack, Text } from "@mantine/core";
import { RobotIcon } from "@phosphor-icons/react";
import { RenderPart } from "@/components/agent-chat/partRegistry";
import type { AgentMessage } from "@/models/agent-chat/AgentMessage";

type AgentMessageViewProps = {
	message: AgentMessage;
};

/** A user prompt rendered as a right-aligned bubble. */
function UserMessage({ message }: AgentMessageViewProps) {
	const text = message.parts.map((part) => (part.type === "text" ? part.text : "")).join("");
	return (
		<Group justify="flex-end">
			<Paper bg="dark.5" px="sm" py={6} radius="lg" style={{ maxWidth: "80%" }}>
				<Text size="sm" style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
					{text}
				</Text>
			</Paper>
		</Group>
	);
}

/** An agent response: an avatar/spinner beside its rendered parts. */
function AgentResponse({ message }: AgentMessageViewProps) {
	const isRunning = message.status === "running";
	return (
		<Group gap="xs" align="flex-start" wrap="nowrap">
			{isRunning ? (
				<Loader size={16} color="grape" />
			) : (
				<RobotIcon size={16} color="var(--mantine-color-grape-4)" />
			)}
			<Paper
				withBorder
				px="sm"
				py={8}
				radius="md"
				style={{ flex: 1, minWidth: 0, borderColor: "var(--mantine-color-dark-4)" }}
			>
				<Stack gap="sm">
					{message.parts.map((part, index) => (
						// Parts are append-only within a message (streamed output only
						// grows and is never reordered), so the index is a stable key.
						// biome-ignore lint/suspicious/noArrayIndexKey: append-only list
						<RenderPart key={`${message.id}-${index}`} part={part} />
					))}
				</Stack>
			</Paper>
		</Group>
	);
}

/** Renders one conversation message, dispatching on its role. */
export function AgentMessageView({ message }: AgentMessageViewProps) {
	if (message.role === "user") return <UserMessage message={message} />;
	return <AgentResponse message={message} />;
}
