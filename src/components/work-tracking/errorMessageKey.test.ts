import { describe, expect, it } from "vitest";
import { InvalidInputReason } from "@/domain/work-tracking/InvalidInputReason";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { WorkTrackingErrorKind } from "@/domain/work-tracking/WorkTrackingErrorKind";
import { en } from "@/i18n/en";
import {
	ERROR_KEYS,
	errorMessageKey,
	RATE_LIMITED_RETRY_AFTER_KEY,
	REASON_KEYS,
} from "./errorMessageKey";

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
		const error = new WorkTrackingError(WorkTrackingErrorKind.Unauthorized, "rejected");

		expect(errorMessageKey(error)).toEqual({ key: "workTracking.errors.unauthorized" });
	});

	it("uses the generic invalid input key without a reason", () => {
		const error = new WorkTrackingError(WorkTrackingErrorKind.InvalidInput, "invalid");

		expect(errorMessageKey(error)).toEqual({ key: "workTracking.errors.invalidInput.generic" });
	});

	it("uses the generic invalid input key for an unknown reason", () => {
		const error = new WorkTrackingError(WorkTrackingErrorKind.InvalidInput, "invalid", {
			reason: "bogus",
		});

		expect(errorMessageKey(error)).toEqual({ key: "workTracking.errors.invalidInput.generic" });
	});

	it("uses the reason key for a known reason", () => {
		const error = new WorkTrackingError(WorkTrackingErrorKind.InvalidInput, "invalid", {
			field: "baseUrl",
			reason: InvalidInputReason.HttpsRequired,
		});

		expect(errorMessageKey(error)).toEqual({
			key: "workTracking.errors.invalidInput.httpsRequired",
		});
	});

	it("includes the retry delay when the provider sent one", () => {
		const error = new WorkTrackingError(WorkTrackingErrorKind.RateLimited, "slow down", {
			retryAfterSeconds: 30,
		});

		expect(errorMessageKey(error)).toEqual({
			key: "workTracking.errors.rateLimitedRetryAfter",
			values: { seconds: 30 },
		});
	});

	it("uses the plain rate limit key without a retry delay", () => {
		const error = new WorkTrackingError(WorkTrackingErrorKind.RateLimited, "slow down");

		expect(errorMessageKey(error)).toEqual({ key: "workTracking.errors.rateLimited" });
	});
});
