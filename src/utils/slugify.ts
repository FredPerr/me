export function slugify(input: string): string {
	return input
		.normalize("NFD") // split accented chars into base + diacritic
		.replace(/[\u0300-\u036f]/g, "") // remove diacritics
		.toLowerCase()
		.trim()
		.replace(/[^a-z0-9]+/g, "-") // any non-alphanumeric run → single hyphen
		.replace(/^-+|-+$/g, ""); // trim leading/trailing hyphens
}
