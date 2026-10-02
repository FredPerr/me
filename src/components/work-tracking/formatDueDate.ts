const DUE_DATE_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/;

/**
 * Formats a `YYYY-MM-DD` due date. The date is built and formatted in UTC so it shows the same
 * calendar day in every time zone. Returns `null` for anything that is not a real calendar date.
 */
export function formatDueDate(dueDate: string | undefined, locale: string): string | null {
	const match = dueDate ? DUE_DATE_PATTERN.exec(dueDate) : null;
	if (!match) return null;
	const date = new Date(Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3])));
	// Date.UTC rolls impossible dates over (2024-02-30 -> March 1) and maps years 0-99 to 19xx,
	// so a round-trip rejects them.
	if (
		date.getUTCFullYear() !== Number(match[1]) ||
		date.getUTCMonth() !== Number(match[2]) - 1 ||
		date.getUTCDate() !== Number(match[3])
	) {
		return null;
	}
	return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeZone: "UTC" }).format(date);
}
