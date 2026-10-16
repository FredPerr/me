import { describe, expect, it } from "vitest";
import type { GitCommandError } from "@/bindings";
import { isCommandError } from "./commandError";

const NO_UPSTREAM_ERROR: GitCommandError = {
	kind: "noUpstream",
	message: "No upstream branch (not pushed yet)",
};
const RAW_MESSAGE = "git push failed";

describe("isCommandError", () => {
	it("accepts an error thrown by a typed command", () => {
		expect(isCommandError(NO_UPSTREAM_ERROR)).toBe(true);
	});

	it("rejects a plain string", () => {
		expect(isCommandError(RAW_MESSAGE)).toBe(false);
	});

	it("rejects a JavaScript error", () => {
		expect(isCommandError(new Error(RAW_MESSAGE))).toBe(false);
	});

	it("rejects an object with a kind but no message", () => {
		expect(isCommandError({ kind: NO_UPSTREAM_ERROR.kind })).toBe(false);
	});

	it("rejects an unrelated object", () => {
		expect(isCommandError({ status: RAW_MESSAGE })).toBe(false);
	});

	it("rejects null", () => {
		expect(isCommandError(null)).toBe(false);
	});
});
