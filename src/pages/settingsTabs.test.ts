import { describe, expect, it } from "vitest";
import { isSettingsTab, SettingsTab, settingsPath } from "./settingsTabs";

const UNKNOWN_TAB = "billing";

describe("settingsTabs", () => {
	it("builds a settings path that opens the given tab", () => {
		expect(settingsPath(SettingsTab.Integrations)).toBe("/settings?tab=integrations");
	});

	it("recognizes known tab values", () => {
		expect(isSettingsTab(SettingsTab.Projects)).toBe(true);
	});

	it("rejects unknown or missing tab values", () => {
		expect(isSettingsTab(UNKNOWN_TAB)).toBe(false);
		expect(isSettingsTab(null)).toBe(false);
	});
});
