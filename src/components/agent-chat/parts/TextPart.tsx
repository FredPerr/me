import { Text } from "@mantine/core";
import type { TextPart as TextPartModel } from "@/models/agent-chat/AgentMessage";

type TextPartProps = {
	part: TextPartModel;
};

/** Renders plain prose with no special structure. */
export function TextPart({ part }: TextPartProps) {
	return (
		<Text size="sm" style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
			{part.text}
		</Text>
	);
}
