import { describe, expect, it } from "vitest";
import type { WorkTrackingCommandError } from "@/bindings";
import { en } from "@/i18n/en";
import {
	ERROR_KEYS,
	errorMessageKey,
	RATE_LIMITED_RETRY_AFTER_KEY,
	REASON_KEYS,
} from "./errorMessageKey";

function buildError(overrides: Partial<WorkTrackingCommandError>): WorkTrackingCommandError {
	return {
		kind: "providerError",
		message: "failed",
		field: null,
		reason: null,
		retryAfterSeconds: null,
		status: null,
		...overrides,
	};
}

function resolveTranslation(key: string): unknown {
	return key
		.split(".")
		.reduce<unknown>(
			(node, segment) =>
				typeof node === "object" && node !== null
					? (node as Record<string, unknown>)[segment]
					: undefined,
			en.translation,
		);
}

describe("errorMessageKey", () => {
	it.each([
		...Object.values(ERROR_KEYS),
		...Object.values(REASON_KEYS),
		RATE_LIMITED_RETRY_AFTER_KEY,
	])("%s resolves to a string in en.ts", (key) => {
		expect(typeof resolveTranslation(key)).toBe("string");
	});

	it("maps every non-special kind to its own key", () => {
		const error = buildError({ kind: "unauthorized" });

		expect(errorMessageKey(error)).toEqual({ key: "workTracking.errors.unauthorized" });
	});

	it("uses the generic invalid input key without a reason", () => {
		const error = buildError({ kind: "invalidInput" });

		expect(errorMessageKey(error)).toEqual({ key: "workTracking.errors.invalidInput.generic" });
	});

	it("uses the reason key for a known reason", () => {
		const error = buildError({ kind: "invalidInput", field: "baseUrl", reason: "httpsRequired" });

		expect(errorMessageKey(error)).toEqual({
			key: "workTracking.errors.invalidInput.httpsRequired",
		});
	});

	it("includes the retry delay when the provider sent one", () => {
		const error = buildError({ kind: "rateLimited", retryAfterSeconds: 30 });

		expect(errorMessageKey(error)).toEqual({
			key: "workTracking.errors.rateLimitedRetryAfter",
			values: { seconds: 30 },
		});
	});

	it("uses the plain rate limit key without a retry delay", () => {
		const error = buildError({ kind: "rateLimited" });

		expect(errorMessageKey(error)).toEqual({ key: "workTracking.errors.rateLimited" });
	});
});
