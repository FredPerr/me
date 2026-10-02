import { Box } from "@mantine/core";
import MDEditor from "@uiw/react-md-editor";
import type { MarkdownPart as MarkdownPartModel } from "@/models/agent-chat/AgentMessage";

type MarkdownPartProps = {
	part: MarkdownPartModel;
};

/**
 * Renders a markdown segment of agent output with formatting (headings, lists,
 * emphasis, inline code). Uses the same markdown renderer as the PR draft view
 * for visual consistency. Code *blocks* are extracted upstream into dedicated
 * code parts, so this handles everything else.
 */
export function MarkdownPart({ part }: MarkdownPartProps) {
	return (
		<Box data-color-mode="dark" fz="sm">
			<MDEditor.Markdown source={part.markdown} style={{ background: "transparent" }} />
		</Box>
	);
}
