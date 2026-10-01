import { describe, expect, it } from "vitest";
import { InvalidInputReason } from "./InvalidInputReason";
import { ProviderKind } from "./ProviderKind";
import { RemoteProjectLink, type RemoteProjectLinkProps } from "./RemoteProjectLink";
import { WorkTrackingError } from "./WorkTrackingError";
import { WorkTrackingErrorKind } from "./WorkTrackingErrorKind";

const LINKED_AT = new Date("2024-01-15T10:00:00.000Z");
const CONNECTION_ID = "teamwork:acme.teamwork.com";
const PROJECT_URL = "https://acme.teamwork.com/app/projects/42";

function buildProps(overrides: Partial<RemoteProjectLinkProps> = {}): RemoteProjectLinkProps {
	return {
		providerKind: ProviderKind.Teamwork,
		connectionId: CONNECTION_ID,
		remoteProjectId: "42",
		remoteProjectName: "Website",
		remoteProjectUrl: PROJECT_URL,
		...overrides,
	};
}

function captureError(action: () => unknown): WorkTrackingError {
	try {
		action();
	} catch (error) {
		if (error instanceof WorkTrackingError) return error;
		throw error;
	}
	throw new Error("Expected a WorkTrackingError");
}

describe("RemoteProjectLink", () => {
	describe("create", () => {
		it("trims the values and records the link time", () => {
			const link = RemoteProjectLink.create(
				buildProps({ remoteProjectId: " 42 ", remoteProjectName: "  Website " }),
				LINKED_AT,
			);

			expect(link.remoteProjectId).toBe("42");
			expect(link.remoteProjectName).toBe("Website");
			expect(link.remoteProjectUrl).toBe(PROJECT_URL);
			expect(link.linkedAt).toBe("2024-01-15T10:00:00.000Z");
		});

		it("rejects an unknown provider kind as unsupported", () => {
			const error = captureError(() =>
				RemoteProjectLink.create(buildProps({ providerKind: "jira" as ProviderKind }), LINKED_AT),
			);

			expect(error.kind).toBe(WorkTrackingErrorKind.InvalidInput);
			expect(error.reason).toBe(InvalidInputReason.Unsupported);
			expect("field" in error).toBe(false);
		});

		it.each([
			["connectionId", { connectionId: "  " }],
			["remoteProjectId", { remoteProjectId: "" }],
			["remoteProjectName", { remoteProjectName: " " }],
		])("rejects an empty %s as required", (_name, overrides) => {
			const error = captureError(() => RemoteProjectLink.create(buildProps(overrides), LINKED_AT));

			expect(error.kind).toBe(WorkTrackingErrorKind.InvalidInput);
			expect(error.message).toBe("Invalid remote project link");
			expect(error.reason).toBe(InvalidInputReason.Required);
			expect("field" in error).toBe(false);
		});

		it("accepts IDs of 300 characters and rejects 301", () => {
			const accepted = RemoteProjectLink.create(
				buildProps({ remoteProjectId: "1".repeat(300) }),
				LINKED_AT,
			);
			const error = captureError(() =>
				RemoteProjectLink.create(buildProps({ connectionId: "c".repeat(301) }), LINKED_AT),
			);

			expect(accepted.remoteProjectId).toHaveLength(300);
			expect(error.reason).toBe(InvalidInputReason.TooLong);
		});

		it("rejects a name longer than 200 characters", () => {
			const error = captureError(() =>
				RemoteProjectLink.create(buildProps({ remoteProjectName: "n".repeat(201) }), LINKED_AT),
			);

			expect(error.reason).toBe(InvalidInputReason.TooLong);
		});

		it.each(["http://acme.teamwork.com/app/projects/42", "javascript:alert(1)", "not a url"])(
			"drops the non-https URL %s",
			(remoteProjectUrl) => {
				const link = RemoteProjectLink.create(buildProps({ remoteProjectUrl }), LINKED_AT);

				expect(link.remoteProjectUrl).toBeUndefined();
				expect("remoteProjectUrl" in link.toJSON()).toBe(false);
			},
		);
	});

	describe("fromJSON", () => {
		it("round-trips through toJSON", () => {
			const link = RemoteProjectLink.create(buildProps(), LINKED_AT);

			const restored = RemoteProjectLink.fromJSON(link.toJSON());

			expect(restored.toJSON()).toEqual(link.toJSON());
		});

		it("rejects an unparseable linkedAt as invalid format", () => {
			const data = { ...RemoteProjectLink.create(buildProps(), LINKED_AT).toJSON(), linkedAt: "x" };

			const error = captureError(() => RemoteProjectLink.fromJSON(data));

			expect(error.reason).toBe(InvalidInputReason.InvalidFormat);
		});
	});

	describe("identity", () => {
		it("is the provider kind, connection and remote project", () => {
			const link = RemoteProjectLink.create(buildProps(), LINKED_AT);

			expect(link.identityKey()).toBe(`teamwork|${CONNECTION_ID}|42`);
		});

		it("ignores the cached name and URL", () => {
			const link = RemoteProjectLink.create(buildProps(), LINKED_AT);
			const renamed = RemoteProjectLink.create(
				buildProps({ remoteProjectName: "Renamed", remoteProjectUrl: undefined }),
				new Date(),
			);
			const other = RemoteProjectLink.create(buildProps({ remoteProjectId: "43" }), LINKED_AT);

			expect(link.sameIdentityAs(renamed)).toBe(true);
			expect(link.sameIdentityAs(other)).toBe(false);
		});
	});

	describe("withCachedDetails", () => {
		it("returns the same instance when nothing changed", () => {
			const link = RemoteProjectLink.create(buildProps(), LINKED_AT);

			expect(link.withCachedDetails("Website", PROJECT_URL)).toBe(link);
		});

		it("returns a refreshed link keeping the identity and linkedAt", () => {
			const link = RemoteProjectLink.create(buildProps(), LINKED_AT);

			const refreshed = link.withCachedDetails("Renamed", undefined);

			expect(refreshed).not.toBe(link);
			expect(refreshed.remoteProjectName).toBe("Renamed");
			expect(refreshed.remoteProjectUrl).toBeUndefined();
			expect(refreshed.linkedAt).toBe(link.linkedAt);
			expect(refreshed.sameIdentityAs(link)).toBe(true);
		});
	});
});
