import { describe, expect, it } from "vitest";
import { WorkTrackingError } from "./WorkTrackingError";
import { WorkTrackingErrorKind } from "./WorkTrackingErrorKind";

describe("WorkTrackingError", () => {
	it("copies only the defined details", () => {
		const error = new WorkTrackingError(WorkTrackingErrorKind.InvalidInput, "Invalid", {
			field: "baseUrl",
			reason: undefined,
		});

		expect(error).toBeInstanceOf(Error);
		expect(error.name).toBe("WorkTrackingError");
		expect(error.kind).toBe(WorkTrackingErrorKind.InvalidInput);
		expect(error.message).toBe("Invalid");
		expect(error.field).toBe("baseUrl");
		expect("reason" in error).toBe(false);
		expect("retryAfterSeconds" in error).toBe(false);
		expect("status" in error).toBe(false);
	});

	describe("fromUnknown", () => {
		it("copies a structured rejection", () => {
			const error = WorkTrackingError.fromUnknown({
				kind: "rateLimited",
				message: "The provider rate limit was reached.",
				retryAfterSeconds: 30,
			});

			expect(error).toBeInstanceOf(WorkTrackingError);
			expect(error.kind).toBe(WorkTrackingErrorKind.RateLimited);
			expect(error.message).toBe("The provider rate limit was reached.");
			expect(error.retryAfterSeconds).toBe(30);
		});

		it("copies the invalid input field and reason", () => {
			const error = WorkTrackingError.fromUnknown({
				kind: "invalidInput",
				message: "The input is invalid.",
				field: "baseUrl",
				reason: "httpsRequired",
			});

			expect(error.field).toBe("baseUrl");
			expect(error.reason).toBe("httpsRequired");
		});

		it("drops wrongly typed fields", () => {
			const error = WorkTrackingError.fromUnknown({
				kind: "providerError",
				message: 42,
				field: 1,
				reason: { value: "x" },
				retryAfterSeconds: "30",
				status: Number.POSITIVE_INFINITY,
			});

			expect(error.kind).toBe(WorkTrackingErrorKind.ProviderError);
			expect(error.message).toBe("");
			expect("field" in error).toBe(false);
			expect("reason" in error).toBe(false);
			expect("retryAfterSeconds" in error).toBe(false);
			expect("status" in error).toBe(false);
		});

		it("maps a string rejection to providerError", () => {
			const error = WorkTrackingError.fromUnknown("Command work_tracking_x not found");

			expect(error.kind).toBe(WorkTrackingErrorKind.ProviderError);
			expect(error.message).toBe("Command work_tracking_x not found");
		});

		it("maps an object with an unknown kind to providerError", () => {
			const error = WorkTrackingError.fromUnknown({ kind: "bogus", message: "x" });

			expect(error.kind).toBe(WorkTrackingErrorKind.ProviderError);
		});
	});
});
