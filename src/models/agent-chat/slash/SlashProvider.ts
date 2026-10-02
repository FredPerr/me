/**
 * Pluggable sources for the chat's slash-command menu.
 *
 * Typing `/` in the chat input opens an autocomplete menu whose entries come
 * from registered {@link SlashProvider}s. Each provider owns one kind of
 * reference — steering files, workspace files, and more later — and turns a
 * query into a list of {@link SlashItem}s. Selecting an item inserts its
 * `insertText` token into the prompt. Adding a new reference kind is adding a
 * provider to the list; the input and menu do not change.
 */

/** A single selectable entry in the slash menu. */
export type SlashItem = {
	/** Stable id, unique across providers (prefix with the provider id). */
	id: string;
	/** Primary label shown in the menu. */
	label: string;
	/** Optional secondary text (e.g. a relative path). */
	description?: string;
	/** The text inserted into the prompt when the item is chosen. */
	insertText: string;
	/** The provider that produced this item, for grouping/icons. */
	providerId: string;
};

/** A source of slash items. */
export type SlashProvider = {
	/** Stable id, e.g. "steering" or "file". */
	readonly id: string;
	/** Human-readable group heading in the menu. */
	readonly title: string;
	/**
	 * Returns items matching `query` (the text typed after `/`, may be empty).
	 * Implementations should filter and cap results themselves.
	 */
	query(query: string): Promise<SlashItem[]>;
};

/** Case-insensitive substring match used by providers to filter candidates. */
export function matchesQuery(candidate: string, query: string): boolean {
	if (query.trim() === "") return true;
	return candidate.toLowerCase().includes(query.toLowerCase());
}
