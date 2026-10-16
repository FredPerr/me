import { describe, expect, it } from "vitest";
import type { GitCommandError } from "@/bindings";
import { describeError, isCommandError } from "./commandError";

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

describe("describeError", () => {
	it("shows the message of a typed command error", () => {
		expect(describeError(NO_UPSTREAM_ERROR)).toBe(NO_UPSTREAM_ERROR.message);
	});

	it("shows the message of a JavaScript error", () => {
		expect(describeError(new Error(RAW_MESSAGE))).toBe(RAW_MESSAGE);
	});

	it("shows a thrown string as is", () => {
		expect(describeError(RAW_MESSAGE)).toBe(RAW_MESSAGE);
	});
});
