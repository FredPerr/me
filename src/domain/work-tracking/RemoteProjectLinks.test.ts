import { afterEach, describe, expect, it, vi } from "vitest";
import type { ConnectionId, WorkProjectId } from "./identifiers";
import { ProviderKind } from "./ProviderKind";
import { RemoteProjectLink } from "./RemoteProjectLink";
import { RemoteProjectLinks } from "./RemoteProjectLinks";

const CONNECTION_ID = "teamwork:acme.teamwork.com";
const FIRST_LINKED_AT = new Date("2024-01-15T10:00:00.000Z");
const LATER = new Date("2024-03-01T08:00:00.000Z");

function buildLink(remoteProjectId: string, remoteProjectName = "Website", now = FIRST_LINKED_AT) {
	return RemoteProjectLink.create(
		{
			providerKind: ProviderKind.Teamwork,
			connectionId: CONNECTION_ID,
			remoteProjectId,
			remoteProjectName,
			remoteProjectUrl: `https://acme.teamwork.com/app/projects/${remoteProjectId}`,
		},
		now,
	);
}

describe("RemoteProjectLinks", () => {
	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe("add", () => {
		it("appends a new link", () => {
			const links = RemoteProjectLinks.empty().add(buildLink("42"));

			expect(links.toArray().map((link) => link.remoteProjectId)).toEqual(["42"]);
			expect(
				links.has(ProviderKind.Teamwork, CONNECTION_ID as ConnectionId, "42" as WorkProjectId),
			).toBe(true);
		});

		it("returns the same instance when the link is already present unchanged", () => {
			const links = RemoteProjectLinks.empty().add(buildLink("42"));

			expect(links.add(buildLink("42", "Website", LATER))).toBe(links);
		});

		it("refreshes the cached details and keeps the original linkedAt", () => {
			const links = RemoteProjectLinks.empty().add(buildLink("42"));

			const refreshed = links.add(buildLink("42", "Renamed", LATER));

			expect(refreshed).not.toBe(links);
			expect(refreshed.toArray()).toHaveLength(1);
			expect(refreshed.toArray()[0].remoteProjectName).toBe("Renamed");
			expect(refreshed.toArray()[0].linkedAt).toBe(FIRST_LINKED_AT.toISOString());
		});

		it("keeps two remote projects linked to the same local project (AC15)", () => {
			const links = RemoteProjectLinks.empty().add(buildLink("42")).add(buildLink("43"));

			expect(links.toArray().map((link) => link.remoteProjectId)).toEqual(["42", "43"]);
		});
	});

	describe("remove", () => {
		it("removes only the given link (AC16)", () => {
			const first = buildLink("42");
			const links = RemoteProjectLinks.empty().add(first).add(buildLink("43"));

			const remaining = links.remove(first.identityKey());

			expect(remaining.toArray().map((link) => link.remoteProjectId)).toEqual(["43"]);
			expect(
				remaining.has(ProviderKind.Teamwork, CONNECTION_ID as ConnectionId, "42" as WorkProjectId),
			).toBe(false);
		});

		it("returns the same instance when the key is absent", () => {
			const links = RemoteProjectLinks.empty().add(buildLink("42"));

			expect(links.remove("teamwork|other|1")).toBe(links);
		});
	});

	describe("fromJSON", () => {
		it.each([undefined, null, "links", { 0: {} }])("returns no links for %s", (data) => {
			expect(RemoteProjectLinks.fromJSON(data).toArray()).toEqual([]);
		});

		it("skips invalid entries with a warning that does not include the data", () => {
			const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
			const valid = buildLink("42").toJSON();

			const links = RemoteProjectLinks.fromJSON([
				null,
				"secret-value",
				{ ...valid, providerKind: "jira" },
				{ ...valid, remoteProjectId: "" },
				valid,
			]);

			expect(links.toArray().map((link) => link.remoteProjectId)).toEqual(["42"]);
			expect(warn).toHaveBeenCalledTimes(4);
			for (const call of warn.mock.calls) {
				expect(call).toEqual(["Skipping invalid remote project link"]);
			}
		});

		it("collapses duplicate identities keeping the first", () => {
			const first = buildLink("42", "First").toJSON();
			const duplicate = buildLink("42", "Second", LATER).toJSON();

			const links = RemoteProjectLinks.fromJSON([first, duplicate, buildLink("43").toJSON()]);

			expect(links.toArray().map((link) => link.remoteProjectName)).toEqual(["First", "Website"]);
		});

		it("round-trips through toJSON", () => {
			const links = RemoteProjectLinks.empty().add(buildLink("42")).add(buildLink("43"));

			expect(RemoteProjectLinks.fromJSON(links.toJSON()).toJSON()).toEqual(links.toJSON());
		});
	});
});
