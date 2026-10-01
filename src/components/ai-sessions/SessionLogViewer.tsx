import { Box, ScrollArea, Text } from "@mantine/core";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { LogLine } from "@/models/ai-session/AiSession";

type SessionLogViewerProps = {
	logs: LogLine[];
	/** When true, the view auto-scrolls to the newest line as logs arrive. */
	follow?: boolean;
};

/**
 * Monospace, scrollable viewer for streamed session output. Stderr lines are
 * tinted so failures stand out from normal output. Auto-follows the tail while
 * the session streams, unless disabled.
 */
export function SessionLogViewer({ logs, follow = true }: SessionLogViewerProps) {
	const { t } = useTranslation();
	const viewportRef = useRef<HTMLDivElement>(null);
	// Read the latest `follow` without it being an effect dependency: a toggle
	// of follow alone should not scroll; only newly arrived logs should.
	const followRef = useRef(follow);
	followRef.current = follow;

	const lineCount = logs.length;
	useEffect(() => {
		// `lineCount` is referenced so the effect re-runs as new lines arrive.
		if (!followRef.current || lineCount === 0) return;
		const viewport = viewportRef.current;
		if (viewport) {
			viewport.scrollTo({ top: viewport.scrollHeight });
		}
	}, [lineCount]);

	if (logs.length === 0) {
		return (
			<Text size="xs" c="dimmed" ff="monospace">
				{t("aiSessions.noOutputYet")}
			</Text>
		);
	}

	return (
		<ScrollArea.Autosize mah={260} type="auto" viewportRef={viewportRef}>
			<Box
				ff="monospace"
				fz="xs"
				style={{ whiteSpace: "pre-wrap", wordBreak: "break-word", lineHeight: 1.5 }}
			>
				{logs.map((line) => (
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
		</ScrollArea.Autosize>
	);
}
