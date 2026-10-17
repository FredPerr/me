import { describe, expect, it } from "vitest";
import { isOpenableProviderUrl } from "./providerUrl";

const BASE_URL = "https://acme.teamwork.com";

describe("isOpenableProviderUrl", () => {
	it("accepts an https URL on the connection host", () => {
		expect(isOpenableProviderUrl("https://acme.teamwork.com/app/tasks/1", BASE_URL)).toBe(true);
	});

	it("compares the port as part of the host", () => {
		expect(
			isOpenableProviderUrl("https://acme.example.com:8443/a", "https://acme.example.com:8443"),
		).toBe(true);
		expect(
			isOpenableProviderUrl("https://acme.example.com/a", "https://acme.example.com:8443"),
		).toBe(false);
	});

	it.each([
		"http://acme.teamwork.com/app/tasks/1",
		"https://evil.example.com/app/tasks/1",
		"https://acme.teamwork.com@evil.example.com/",
		"javascript:alert(1)",
		"not a url",
		"",
	])("rejects %s", (url) => {
		expect(isOpenableProviderUrl(url, BASE_URL)).toBe(false);
	});

	it("rejects everything when the base URL is invalid", () => {
		expect(isOpenableProviderUrl("https://acme.teamwork.com/app", "not a url")).toBe(false);
	});
});
