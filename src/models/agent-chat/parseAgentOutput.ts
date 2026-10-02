/**
 * Parses an agent's streamed plain-text output into typed renderable parts.
 *
 * Today a CLI run yields a flat stream of text; this turns that text into a
 * sequence of {@link MessagePart}s the renderer registry can display richly.
 * It recognizes fenced code blocks (``` with an optional language) and emits
 * everything else as markdown parts (the markdown renderer also handles plain
 * prose, headings, lists, and inline code). When structured events arrive from
 * a future backend, those parts are produced upstream and merged separately;
 * this function is only responsible for the free-text portion.
 */

import type { CodeBlockPart, MarkdownPart, MessagePart } from "@/models/agent-chat/AgentMessage";

/** Matches a fenced code block: ```lang\n ... \n``` (language optional). */
const FENCE = /```([^\n`]*)\n([\s\S]*?)```/g;

function markdownPart(markdown: string): MarkdownPart {
	return { type: "markdown", markdown };
}

function codePart(code: string, language: string): CodeBlockPart {
	const trimmedLanguage = language.trim();
	return {
		type: "code",
		code: code.replace(/\n$/, ""),
		...(trimmedLanguage ? { language: trimmedLanguage } : {}),
	};
}

/**
 * Splits raw text into alternating markdown and code parts. Blank or
 * whitespace-only markdown segments between/around fences are dropped so the
 * transcript does not render empty blocks.
 */
export function parseAgentOutput(rawText: string): MessagePart[] {
	if (rawText.trim() === "") return [];

	const parts: MessagePart[] = [];
	let lastIndex = 0;

	FENCE.lastIndex = 0;
	let match = FENCE.exec(rawText);
	while (match !== null) {
		const precedingText = rawText.slice(lastIndex, match.index);
		if (precedingText.trim() !== "") parts.push(markdownPart(precedingText.trim()));

		parts.push(codePart(match[2], match[1]));
		lastIndex = match.index + match[0].length;
		match = FENCE.exec(rawText);
	}

	const trailingText = rawText.slice(lastIndex);
	if (trailingText.trim() !== "") parts.push(markdownPart(trailingText.trim()));

	return parts;
}
