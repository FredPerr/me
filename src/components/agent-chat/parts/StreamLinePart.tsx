import { Text } from "@mantine/core";
import type { StreamLinePart as StreamLinePartModel } from "@/models/agent-chat/AgentMessage";

type StreamLinePartProps = {
	part: StreamLinePartModel;
};

/** Renders a single raw terminal line, tagged red for stderr. */
export function StreamLinePart({ part }: StreamLinePartProps) {
	return (
		<Text
			component="div"
			ff="monospace"
			fz="xs"
			c={part.stream === "stderr" ? "red.4" : undefined}
			style={{ whiteSpace: "pre-wrap", wordBreak: "break-word", lineHeight: 1.5 }}
		>
			{part.text}
		</Text>
	);
}
