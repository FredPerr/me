import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
	ConnectionId,
	WorkItemGroupId,
	WorkProjectId,
} from "@/domain/work-tracking/identifiers";
import { ProviderKind } from "@/domain/work-tracking/ProviderKind";
import { WorkTrackingError } from "@/domain/work-tracking/WorkTrackingError";
import { WorkTrackingErrorKind } from "@/domain/work-tracking/WorkTrackingErrorKind";
import { TauriWorkTrackingGateway } from "./TauriWorkTrackingGateway";

vi.mock("@tauri-apps/api/core", () => ({
	invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);
const CONNECTION_ID = "teamwork:acme.teamwork.com" as ConnectionId;
const PROJECT_ID = "42" as WorkProjectId;
const GROUP_ID = "7" as WorkItemGroupId;
const EMPTY_PAGE = { items: [] };

describe("TauriWorkTrackingGateway", () => {
	const gateway = new TauriWorkTrackingGateway();

	beforeEach(() => {
		invokeMock.mockReset();
	});

	it("lists connections", async () => {
		const connections = [
			{
				id: CONNECTION_ID,
				kind: "teamwork",
				baseUrl: "https://acme.teamwork.com",
				displayName: "acme.teamwork.com",
				credentialConfigured: true,
			},
		];
		invokeMock.mockResolvedValueOnce(connections);

		const result = await gateway.listConnections();

		expect(result).toEqual(connections);
		expect(invokeMock).toHaveBeenCalledWith("work_tracking_list_connections", undefined);
	});

	it("saves a connection with exactly the four input fields", async () => {
		invokeMock.mockResolvedValueOnce({});

		await gateway.saveConnection({
			kind: ProviderKind.Teamwork,
			baseUrl: "https://acme.teamwork.com",
			displayName: "Acme",
			apiKey: "secret-key",
		});

		expect(invokeMock).toHaveBeenCalledWith("work_tracking_save_connection", {
			input: {
				kind: "teamwork",
				baseUrl: "https://acme.teamwork.com",
				displayName: "Acme",
				apiKey: "secret-key",
			},
		});
	});

	it("removes a connection", async () => {
		invokeMock.mockResolvedValueOnce(null);

		await expect(gateway.removeConnection(CONNECTION_ID)).resolves.toBeUndefined();

		expect(invokeMock).toHaveBeenCalledWith("work_tracking_remove_connection", {
			connectionId: CONNECTION_ID,
		});
	});

	it("lists projects with a null cursor for the first page", async () => {
		invokeMock.mockResolvedValueOnce(EMPTY_PAGE);

		await gateway.listProjects(CONNECTION_ID);

		expect(invokeMock).toHaveBeenCalledWith("work_tracking_list_projects", {
			connectionId: CONNECTION_ID,
			cursor: null,
		});
	});

	it("lists groups with the given cursor", async () => {
		invokeMock.mockResolvedValueOnce(EMPTY_PAGE);

		await gateway.listGroups(CONNECTION_ID, PROJECT_ID, "2");

		expect(invokeMock).toHaveBeenCalledWith("work_tracking_list_groups", {
			connectionId: CONNECTION_ID,
			projectId: PROJECT_ID,
			cursor: "2",
		});
	});

	it("lists items of a group", async () => {
		const page = { items: [], nextCursor: "3" };
		invokeMock.mockResolvedValueOnce(page);

		const result = await gateway.listItems(CONNECTION_ID, PROJECT_ID, GROUP_ID);

		expect(result).toEqual(page);
		expect(invokeMock).toHaveBeenCalledWith("work_tracking_list_items", {
			connectionId: CONNECTION_ID,
			projectId: PROJECT_ID,
			groupId: GROUP_ID,
			cursor: null,
		});
	});

	it("rethrows a structured rejection as a typed WorkTrackingError", async () => {
		invokeMock.mockRejectedValueOnce({
			kind: "invalidInput",
			message: "The input is invalid.",
			field: "baseUrl",
			reason: "httpsRequired",
		});

		const result = gateway.saveConnection({
			kind: ProviderKind.Teamwork,
			baseUrl: "http://acme.teamwork.com",
			apiKey: "secret-key",
		});

		await expect(result).rejects.toBeInstanceOf(WorkTrackingError);
		await expect(result).rejects.toMatchObject({
			kind: WorkTrackingErrorKind.InvalidInput,
			field: "baseUrl",
			reason: "httpsRequired",
		});
	});

	it("maps a string rejection to providerError", async () => {
		invokeMock.mockRejectedValueOnce("invalid args `input` for command");

		await expect(gateway.listProjects(CONNECTION_ID)).rejects.toMatchObject({
			kind: WorkTrackingErrorKind.ProviderError,
			message: "invalid args `input` for command",
		});
	});
});
