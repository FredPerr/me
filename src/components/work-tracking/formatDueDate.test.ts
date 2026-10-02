import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { formatDueDate } from "./formatDueDate";

describe("formatDueDate", () => {
	let originalTimeZone: string | undefined;

	beforeEach(() => {
		originalTimeZone = process.env.TZ;
		process.env.TZ = "America/Toronto";
	});

	afterEach(() => {
		if (originalTimeZone === undefined) delete process.env.TZ;
		else process.env.TZ = originalTimeZone;
	});

	it("shows the same calendar day west of UTC", () => {
		const formatted = formatDueDate("2024-01-15", "en-US");

		expect(formatted).toContain("15");
		expect(formatted).toContain("2024");
	});

	it.each([
		["an impossible calendar date", "2024-02-30"],
		["a date without zero padding", "2024-1-5"],
		["a date with a time", "2024-02-30T00:00"],
		["a two-digit year that Date.UTC would shift", "0024-01-15"],
		["an empty string", ""],
		["no date", undefined],
	])("returns null for %s", (_description, dueDate) => {
		expect(formatDueDate(dueDate, "en-US")).toBeNull();
	});
});
