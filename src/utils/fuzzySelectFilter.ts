import type { ComboboxItem, ComboboxParsedItem, OptionsFilter } from "@mantine/core";
import { fuzzyMatch } from "./fuzzyMatch";

function isComboboxItem(item: ComboboxParsedItem): item is ComboboxItem {
	return "value" in item;
}

export const fuzzySelectFilter: OptionsFilter = ({ options, search }) => {
	const scored = options
		.filter(isComboboxItem)
		.map((option) => ({ option, score: fuzzyMatch(option.label, search) }))
		.filter(({ score }) => score >= 0)
		.sort((a, b) => b.score - a.score);

	return scored.map(({ option }) => option);
};
