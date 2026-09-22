import type { OrderedReviewItem, ReviewItem } from "./ReviewStack";

/**
 * Formats an ordered list of review items into a Slack message.
 *
 * Slack renders links as `<url|label>` and does not render nested markdown
 * bullets richly, so stack depth is conveyed with plain-space indentation and
 * a chain marker. Dependencies are noted inline by PR number.
 */

const INDENT = "    ";
const CHAIN_MARKER = "└ ";

function dependencyNote(item: ReviewItem, includedIds: Set<string>): string {
	const included = item.dependsOn.filter((dependencyId) => includedIds.has(dependencyId));
	if (included.length === 0) return "";
	const numbers = included.map((dependencyId) => dependencyId.replace(/^#?/, "#")).join(", ");
	return ` (depends on ${numbers})`;
}

function formatLine(ordered: OrderedReviewItem, includedIds: Set<string>): string {
	const { item, depth } = ordered;
	const indent = INDENT.repeat(depth);
	const marker = depth > 0 ? CHAIN_MARKER : "";
	const link = `<${item.url}|#${item.number} ${item.title}>`;
	return `${indent}${marker}${link}${dependencyNote(item, includedIds)}`;
}

/**
 * Build the full Slack message. An optional `heading` is placed on the first
 * line. Returns an empty string when there is nothing to share.
 */
export function formatSlackMessage(ordered: OrderedReviewItem[], heading?: string): string {
	if (ordered.length === 0) return "";

	const includedIds = new Set(ordered.map((entry) => entry.item.id));
	const lines = ordered.map((entry) => formatLine(entry, includedIds));

	return heading ? [heading, ...lines].join("\n") : lines.join("\n");
}
